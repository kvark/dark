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

    fn encode<W: io::Write>(&mut self, _sym: Symbol, _ctx: &Context, _eh: &mut ari::Encoder<W>) {
        //TODO
    }

    fn decode<R: io::Read>(&mut self, _ctx: &Context, _dh: &mut ari::Decoder<R>) -> Symbol {
        0 //TODO
    }
}
