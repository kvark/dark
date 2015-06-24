/*!

YBS-like compression model

Work in progress, based on the notes from Vadim.

# Links

http://www.compression.ru/ybs/

# Credit

Vadim Yoockin for sharing details of YBS implementation.

*/

use std::{cmp, io};
use std::vec::Vec;
use compress::entropy::ari;


struct SymbolContext {
    pub avg_log     : usize,
    pub last_diff   : usize,
}

impl SymbolContext {
    fn new() -> SymbolContext {
        SymbolContext{ avg_log:0, last_diff:0 }
    }

    fn update(&mut self, log: u32) {
        let a = if self.last_diff>3 {2u32} else {1u32};
        let b = 1u32;
        self.last_diff = ((log as isize) - (self.avg_log as isize)).abs() as usize;
        self.avg_log = (a*log + b*self.avg_log) / (a+b);
    }
}


/// Coding model for BWT-DC output
pub struct Model {
    table_log   : Vec<ari::table::Model>,
    table_high  : ari::table::Model,
    bin_rest    : [ari::bin::Model; 3],
    /// specific context tracking
    contexts    : [SymbolContext; 0x100],
}

impl Model {
    /// Create a new Model instance
    pub fn new(threshold: ari::Border) -> Model {
        let low_groups = 13u32;
        Model {
            table_log   : Vec::from_fn(low_groups, |_| ari::table::Model::new_flat(low_groups+1, threshold)),
            table_high  : ari::table::Model::new_flat(32-low_groups, threshold),
            bin_rest    : [ari::bin::Model::new_flat(threshold, 5); 3],
            contexts    : [SymbolContext::new(); 0x100],
        }
    }
}

impl super::DistanceModel for Model {
    fn new_default() -> Model {
        Model::new(ari::RANGE_DEFAULT_THRESHOLD >> 2)
    }

    fn reset(&mut self) {
        for table in self.table_log.mut_iter() {
            table.reset_flat();
        }
        self.table_high.reset_flat();
        for bm in self.bin_rest.mut_iter() {
            bm.reset_flat();
        }
        for con in self.contexts.mut_iter() {
            con.avg_log = 0;
            con.last_diff = 0;
        }
    }

    fn encode<W: io::Write>(&mut self, dist: super::Distance, ctx: &super::Context, eh: &mut ari::Encoder<W>) {
        let max_low_log = self.table_log.len()-1;
        let group = if dist<4 {
            dist as usize
        } else {
            let mut log = 3;
            while dist>>log !=0 {log+=1;}
            (log+1) as usize
        };
        let context = &mut self.contexts[ctx.symbol as usize];
        let con_log = cmp::min(context.avg_log, max_low_log);
        let freq_log = self.table_log.get_mut(con_log);
        // write exponent
        let log_encoded = cmp::min(group, max_low_log);
        eh.encode(log_encoded, freq_log).unwrap();
        // update model
        freq_log.update(log_encoded, 10, 1);
        context.update(log_encoded);    //use log?
        if group<4 {
            return
        }
        if group >= max_low_log {
            let add = group - max_low_log;
            eh.encode(add, &self.table_high).unwrap();
            self.table_high.update(add, 10, 1);
        }
        // write mantissa
        let log = group-1;
        for i in 1 .. log {
            let bit = (dist>>(log-i-1)) as usize & 1 != 0;
            if i >= self.bin_rest.len() {
                // just send bits past the model, equally distributed
                eh.encode(bit, self.bin_rest.last().unwrap()).unwrap();
            }else {
                let bc = &mut self.bin_rest[i-1];
                eh.encode(bit, bc).unwrap();
                bc.update(bit);
            };
        }
    }

    fn decode<R: io::Read>(&mut self, ctx: &super::Context, dh: &mut ari::Decoder<R>) -> super::Distance {
        let max_low_log = self.table_log.len()-1;
        let context = &mut self.contexts[ctx.symbol as usize];
        let con_log = cmp::min(context.avg_log, max_low_log);
        let freq_log = self.table_log.get_mut(con_log);
        // read exponent
        let log_decoded = dh.decode(freq_log).unwrap();
        // update model
        context.update(log_decoded);
        freq_log.update(log_decoded, 10, 1);
        if log_decoded < 4 {
            return log_decoded as super::Distance
        }
        let group = if log_decoded == max_low_log {
            let add = dh.decode(&self.table_high).unwrap();
            self.table_high.update(add, 10, 1);
            max_low_log + add
        }else {log_decoded};
        // read mantissa
        let log = group - 1;
        let mut dist = 1 as super::Distance;
        for i in 1 .. log {
            let bit = if i >= self.bin_rest.len() {
                dh.decode( self.bin_rest.last().unwrap() ).unwrap()
            }else {
                let bc = &mut self.bin_rest[i-1];
                let bit = dh.decode(bc).unwrap();
                bc.update(bit);
                bit
            };
            dist = (dist<<1) + (bit as super::Distance);
        }
        dist
    }
}
