/*!

bbb compression model

# Links

* http://mattmahoney.net/dc/bbb.cpp

*/

use std::io;
use compress::entropy::ari;
use super::Symbol;


/// Symbol encoding context //TODO
pub type Context = ();

const FIXED_BASE    : u32 = 8;
const LOG_LIMIT     : usize = 10;
const LOG_DEFAULT   : u32 = 1<<FIXED_BASE;

/// Coding model for BWT-DC output
pub struct Model {
    next_prob: ari::apm::Bit,
    avg_log: [u32; 0x100],   //fixed-point
    prob: [[ari::apm::Bit; 24]; LOG_LIMIT],
}

impl Model {
    /// Create a new model
    pub fn new() -> Model {
        Model {
            next_prob: ari::apm::Bit::new_equal(),
            avg_log: [LOG_DEFAULT; 0x100],
            prob: [[ari::apm::Bit::new_equal(); 24]; LOG_LIMIT],
        }
    }

    fn update(&mut self, bit: u8) {
        //TODO
    }

    fn predict(&self) -> ari::apm::Bit {
        if self.next_prob.predict() {
            self.next_prob.clone()
        }else {
            // weird hack by MM, TODO: investigate
            let flat = self.next_prob.to_flat() + 1;
            ari::apm::Bit::from_flat(flat)
        }
    }
}

impl super::Model<Symbol, Context> for Model {
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

    fn encode<W: io::Write>(&mut self, sym: Symbol, _ctx: &Context,
              eh: &mut ari::Encoder<W>) -> io::Result<()> {
        for i in (0..8).rev() {
            let bit = (sym >> i) & 1;
            try!(eh.encode(bit != 0, &self.predict()));
            self.update(bit);
        }
        Ok(())
    }

    fn decode<R: io::Read>(&mut self, _ctx: &Context, dh: &mut ari::Decoder<R>)
              -> io::Result<Symbol> {
        let mut sym = 0 as Symbol;
        for i in (0..8).rev() {
            let bit_b = try!(dh.decode( &self.predict() ));
            let bit = if bit_b {1} else {0};
            self.update(bit);
            sym |= bit << i;
        }
        Ok(sym)
    }
}
