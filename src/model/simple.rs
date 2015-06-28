/*!

A simple coding model to be a baseline for comparison

*/

use std::{cmp, io};
use compress::bwt::dc::Context;
use compress::entropy::ari;
use super::Distance;


/// A pass-though byte frequency model
pub struct Raw;
impl ari::Model<u8> for Raw {
    fn get_range(&self, value: u8) -> (ari::Border, ari::Border) {
        (value as ari::Border, value as ari::Border+1)
    }
    fn find_value(&self, offset: ari::Border) -> (u8, ari::Border, ari::Border) {
        (offset as u8, offset, offset+1)
    }
    fn get_denominator(&self) -> ari::Border {0x100}
}


/// A simple DC model, coding up to 0xFF distances as-is, and with following 3 bytes otherwise
pub struct Model {
    freq: Vec<ari::table::Model>,
    up  : [usize; 4],
}

impl Model {
    /// Create a new Model
    pub fn new() -> Model {
        let threshold = ari::RANGE_DEFAULT_THRESHOLD >> 2;
        Model {
            freq: (0..4).map(|_| ari::table::Model::new_flat(0x100, threshold)).collect(),
            up  : [10,8,7,6],
        }
    }
}

impl super::Model<Distance, Context> for Model {
    fn reset(&mut self) {
        for table in self.freq.iter_mut() {
            table.reset_flat();
        }
    }

    fn encode<W: io::Write>(&mut self, dist: Distance, _ctx: &Context, eh: &mut ari::Encoder<W>) {
        let val = cmp::min(0xFF, dist) as usize;
        eh.encode(val, &self.freq[0]).unwrap();
        self.freq[0].update(val, self.up[0], 1);
        if val == 0xFF {
            let rest = (dist - 0xFF) as usize;
            for i in 0usize .. 3 {
                let b = (rest>>(i*8))&0xFF;
                eh.encode(b, &self.freq[i+1]).unwrap();
                self.freq[i+1].update(b, self.up[i+1], 1);
            }
        }
    }

    fn decode<R: io::Read>(&mut self, _ctx: &Context, dh: &mut ari::Decoder<R>) -> Distance {
        let base = dh.decode(&self.freq[0]).unwrap();
        self.freq[0].update(base, self.up[0], 1);
        let d = if base == 0xFF {
            (0usize .. 3).fold(base, |u,i| {
                let b = dh.decode(&self.freq[i+1]).unwrap();
                self.freq[i+1].update(b, self.up[i+1], 1);
                u + (b<<(i*8))
            })
        }else {base};
        d as Distance
    }
}
