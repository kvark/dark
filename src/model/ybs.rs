/*!

YBS-like compression model

Work in progress, based on the notes from Vadim.

# Links

http://www.compression.ru/ybs/

# Credit

Vadim Yoockin for sharing details of YBS implementation.

*/

use std::{cmp, io};
use compress::bwt::dc::Context;
use compress::entropy::ari;
use super::Distance;


struct SymbolContext {
    pub avg_log     : usize,
    pub last_diff   : usize,
}

impl SymbolContext {
    fn new() -> SymbolContext {
        SymbolContext{ avg_log:0, last_diff:0 }
    }

    fn update(&mut self, log: usize) {
        let a = if self.last_diff>3 {2usize} else {1usize};
        let b = 1usize;
        self.last_diff = ((log as isize) - (self.avg_log as isize)).abs() as usize;
        self.avg_log = (a*log + b*self.avg_log) / (a+b);
    }
}


/// Coding model for BWT-DC output
pub struct Model {
    table_log   : Vec<ari::table::Model>,
    table_high  : ari::table::Model,
    bin_rest    : Vec<ari::bin::Model>,
    /// specific context tracking
    contexts    : Vec<SymbolContext>,
}

impl Model {
    /// Create a new Model instance
    pub fn new_custom(threshold: ari::Border) -> Model {
        let low_groups = 13usize;
        Model {
            table_log   : (0..low_groups).map(|_|
                ari::table::Model::new_flat(low_groups+1, threshold)
                ).collect(),
            table_high  : ari::table::Model::new_flat(32-low_groups, threshold),
            bin_rest    : (0..3).map(|_|
                ari::bin::Model::new_flat(threshold, 5)
                ).collect(),
            contexts    : (0..0x100).map(|_| SymbolContext::new()).collect(),
        }
    }

    /// Create a new default Model
    pub fn new() -> Model {
        Model::new_custom(ari::RANGE_DEFAULT_THRESHOLD >> 2)
    }
}

impl super::Model<Distance, Context> for Model {
    fn reset(&mut self) {
        for table in self.table_log.iter_mut() {
            table.reset_flat();
        }
        self.table_high.reset_flat();
        for bm in self.bin_rest.iter_mut() {
            bm.reset_flat();
        }
        for con in self.contexts.iter_mut() {
            con.avg_log = 0;
            con.last_diff = 0;
        }
    }

    fn encode<W: io::Write>(&mut self, dist: Distance, ctx: &Context,
              eh: &mut ari::Encoder<W>) -> io::Result<()> {
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
        let freq_log = &mut self.table_log[con_log];
        // write exponent
        let log_encoded = cmp::min(group, max_low_log);
        try!(eh.encode(log_encoded, freq_log));
        // update model
        freq_log.update(log_encoded, 10, 1);
        context.update(log_encoded);    //use log?
        if group<4 {
            return Ok(())
        }
        if group >= max_low_log {
            let add = group - max_low_log;
            try!(eh.encode(add, &self.table_high));
            self.table_high.update(add, 10, 1);
        }
        // write mantissa
        let log = group-1;
        for i in 1 .. log {
            let bit = (dist>>(log-i-1)) as usize & 1 != 0;
            if i >= self.bin_rest.len() {
                // just send bits past the model, equally distributed
                try!(eh.encode(bit, self.bin_rest.last().unwrap()));
            }else {
                let bc = &mut self.bin_rest[i-1];
                try!(eh.encode(bit, bc));
                bc.update(bit);
            };
        }
        Ok(())
    }

    fn decode<R: io::Read>(&mut self, ctx: &Context, dh: &mut ari::Decoder<R>)
              -> io::Result<Distance> {
        let max_low_log = self.table_log.len()-1;
        let context = &mut self.contexts[ctx.symbol as usize];
        let con_log = cmp::min(context.avg_log, max_low_log);
        let freq_log = &mut self.table_log[con_log];
        // read exponent
        let log_decoded = try!(dh.decode(freq_log));
        // update model
        context.update(log_decoded);
        freq_log.update(log_decoded, 10, 1);
        if log_decoded < 4 {
            return Ok(log_decoded as Distance)
        }
        let group = if log_decoded == max_low_log {
            let add = try!(dh.decode(&self.table_high));
            self.table_high.update(add, 10, 1);
            max_low_log + add
        }else {log_decoded};
        // read mantissa
        let log = group - 1;
        let mut dist = 1 as super::Distance;
        for i in 1 .. log {
            let bit = if i >= self.bin_rest.len() {
                try!(dh.decode( self.bin_rest.last().unwrap() ))
            }else {
                let bc = &mut self.bin_rest[i-1];
                let bit = try!(dh.decode(bc));
                bc.update(bit);
                bit
            };
            dist = (dist<<1) + (bit as Distance);
        }
        Ok(dist)
    }
}
