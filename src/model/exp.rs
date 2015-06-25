/*!

Experimental BWT-DC compression model

*/

use std::io;
use compress::entropy::ari;


const FIXED_BASE    : u32 = 8;
const FIXED_MASK    : u32 = (1<<FIXED_BASE) - 1;
const LOG_LIMIT     : usize = 10;
const LOG_DEFAULT   : u32 = 1<<FIXED_BASE;
const BIT_UPDATE    : isize = 5;

/// Coding model for BWT-DC output
pub struct Model {
    avg_log : [u32; 0x100],   //fixed-point
    prob    : [[ari::apm::Bit; 24]; LOG_LIMIT],
}

impl Model {
    /// Create a new model
    pub fn new() -> Model {
        Model {
            avg_log : [LOG_DEFAULT; 0x100],
            prob    : [[ari::apm::Bit::new_equal(); 24]; LOG_LIMIT],
        }
    }

    fn get_log(d: super::Distance) -> u32 {
        let du = d as u32;
        match d {
            0...2   => du << FIXED_BASE,
            3...4   => (3 << FIXED_BASE) + (du&1)*(FIXED_BASE>>1),
            5...7   => (4 << FIXED_BASE) + ((du-5)%3)*(FIXED_BASE/3),
            8...12  => (5 << FIXED_BASE) + (du&3)*(FIXED_BASE>>2),
            _       => (6 << FIXED_BASE) + (du-12)*(FIXED_BASE>>12),
        }
    }
}

impl super::DistanceModel for Model {
    fn new_default() -> Model {
        Model::new()
    }

    fn reset(&mut self) {
        for log in self.avg_log.iter_mut() {
            *log = LOG_DEFAULT;
        }
        for log in self.prob.iter_mut() {
            for bit in log.iter_mut() {
                *bit = ari::apm::Bit::new_equal();
            }
        }
    }

    fn encode<W: io::Write>(&mut self, dist: super::Distance, ctx: &super::Context, eh: &mut ari::Encoder<W>) {
        // find context
        let log = self.avg_log[ctx.symbol as usize];
        let w2 = log & FIXED_MASK;
        let w1 = FIXED_MASK + 1 - w2;
        let (pr1,pr2) = self.prob.split_at_mut((log>>FIXED_BASE) as usize + 1);
        let (m1,m2) = (pr1.last_mut().unwrap(), &mut pr2[0]);
        // encode
        for (i,(b1,b2)) in m1.iter_mut().zip(m2.iter_mut()).enumerate().rev() {
            let value = dist & (1<<i) != 0;
            let flat = (w1 * (b1.to_flat() as u32) + w2 * (b2.to_flat() as u32)) >> FIXED_BASE;
            let bit = ari::apm::Bit::from_flat(flat as ari::apm::FlatProbability);
            eh.encode(value, &bit).unwrap();
            b1.update(value, BIT_UPDATE, 0);
            b2.update(value, BIT_UPDATE, 0);
        }
        // update
        self.avg_log[ctx.symbol as usize] = (3*log + Model::get_log(dist)) >> 2;
    }

    fn decode<R: io::Read>(&mut self, ctx: &super::Context, dh: &mut ari::Decoder<R>) -> super::Distance {
        // find context
        let log = self.avg_log[ctx.symbol as usize];
        let w2 = log & FIXED_MASK;
        let w1 = FIXED_MASK + 1 - w2;
        let (pr1,pr2) = self.prob.split_at_mut((log>>FIXED_BASE) as usize + 1);
        let (m1,m2) = (pr1.last_mut().unwrap(), &mut pr2[0]);
        // decode
        let dist = m1.iter_mut().zip(m2.iter_mut()).rev().fold(0 as super::Distance, |u,(b1,b2)| {
            let flat = (w1 * (b1.to_flat() as u32) + w2 * (b2.to_flat() as u32)) >> FIXED_BASE;
            let bit = ari::apm::Bit::from_flat(flat as ari::apm::FlatProbability);
            let value = dh.decode(&bit).unwrap();
            b1.update(value, BIT_UPDATE, 0);
            b2.update(value, BIT_UPDATE, 0);
            (u<<1) + if value {1} else {0}
        });
        // update
        self.avg_log[ctx.symbol as usize] = (3*log + Model::get_log(dist)) >> 2;
        dist
    }
}
