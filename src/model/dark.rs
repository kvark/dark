/*!

Dark-0.51 exact compression model

# Links

http://darchiver.narod.ru/
http://code.google.com/p/adark/

*/

use std::{cmp, io};
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use num::{Float, NumCast};
use compress::entropy::ari;


/// Aggregate frequency model of two sources
pub struct Aggregate<'a, F, X: 'a, Y: 'a> {
    x: &'a X,
    y: &'a Y,
    dummy: PhantomData<F>,
}

impl<'a, F: Float + Debug + Display, X: ari::Model<F>, Y: ari::Model<F>>
ari::Model<F> for Aggregate<'a, F, X, Y> {
    fn get_range(&self, value: F) -> (ari::Border,ari::Border) {
        let (x1,x2) = self.x.get_range(value);
        let (y1,y2) = self.y.get_range(value);
        (x1+y1, x2+y2)
    }

    fn find_value(&self, _offset: ari::Border) -> (F,ari::Border,ari::Border) {
        (NumCast::from(0).unwrap(), 0,0)    //TODO
    }

    fn get_denominator(&self) -> ari::Border {
        self.x.get_denominator() + self.y.get_denominator()
    }
}


const MAX_LOG_CODE     : usize = 8;
const MAX_LOG_CONTEXT  : usize = 11;
const NUM_LAST_LOGS    : u32 = 3;
const MAX_BIT_CONTEXT  : usize = 3;
const ADAPT_POWERS     : [isize; 9] = [6,5,4,3,2,1,4,6,4];


struct BinaryMultiplex {
    pub freqs: Vec<ari::bin::Model>,
}

impl BinaryMultiplex {
    fn new(threshold: ari::Border, factor: ari::Border) -> BinaryMultiplex {
        BinaryMultiplex {
            freqs: (0..32).map(|_|
                ari::bin::Model::new_flat(threshold, factor)
                ).collect(),
        }
    }
    fn reset(&mut self) {
        for fr in self.freqs.iter_mut() {
            fr.reset_flat();
        }
    }
}


struct SymbolContext {
    pub avg_dist    : isize,
    pub freq_log    : ari::table::Model,
    pub freq_extra  : BinaryMultiplex,
}

impl SymbolContext {
    fn new(threshold: ari::Border, factor: ari::Border) -> SymbolContext {
        SymbolContext{
            avg_dist    : 1000,
            freq_log    : ari::table::Model::new_flat(MAX_LOG_CODE, threshold),
            freq_extra  : BinaryMultiplex::new(threshold, factor),
        }
    }

    fn reset(&mut self) {
        self.avg_dist = 1000;
        self.freq_log.reset_flat();
        self.freq_extra.reset();
    }

    fn update(&mut self, dist: super::Distance, log_diff: isize) {
        let adapt = if log_diff < -6 {7}
        else if log_diff >= 3 {3}
        else { ADAPT_POWERS[(6+log_diff) as usize] };
        self.avg_dist += (adapt*((dist as isize) - self.avg_dist)) >> 3;
        debug!("\tUpdated avg_dist to {}, using raz {}, dist {} and power {}",
            self.avg_dist, log_diff, dist, adapt);
    }
}


/// Coding model for BWT-DC output
pub struct Model {
    freq_log: Vec<Vec<ari::table::Model>>,  //[MAX_LOG_CONTEXT+1][NUM_LAST_LOGS]
    freq_log_bits: [BinaryMultiplex; 2],
    freq_mantissa: Vec<Vec<ari::bin::Model>>,    //[32][MAX_BIT_CONTEXT+1]
    /// specific context tracking
    contexts: Vec<SymbolContext>,
    last_log_token: usize,
    /// update parameters
    update_log_global: usize,
    update_log_power: usize,
    update_log_add: ari::Border,
}

impl Model {
    /// Create a new Model instance
    pub fn new(threshold: ari::Border) -> Model {
        Model {
            freq_log: (0 .. MAX_LOG_CONTEXT + 1).map(|_|
                (0..NUM_LAST_LOGS).map(|_|
                    ari::table::Model::new_flat(MAX_LOG_CODE, threshold)
                    ).collect()
                ).collect(),
            freq_log_bits: [
                BinaryMultiplex::new(threshold, 2),
                BinaryMultiplex::new(threshold, 2)
                ],
            freq_mantissa: (0..32).map(|_|
                (0 .. MAX_BIT_CONTEXT + 1).map(|_|
                    ari::bin::Model::new_flat(threshold, 8)
                    ).collect()
                ).collect(),
            contexts: (0..0x100).map(|_|
                SymbolContext::new(threshold, 3)
                ).collect(),
            last_log_token: 1,
            update_log_global: 12,
            update_log_power: 5,
            update_log_add: 5,
        }
    }

    fn isize_log(d: super::Distance) -> usize {
        let mut log = 0;
        while d>>log !=0 {log += 1;}
        log
    }
}

impl super::DistanceModel for Model {
    fn new_default() -> Model {
        Model::new(ari::RANGE_DEFAULT_THRESHOLD >> 2)
    }

    fn reset(&mut self) {
        for array in self.freq_log.iter_mut() {
            for table in array.iter_mut() {
                table.reset_flat();
            }
        }
        for bm in self.freq_log_bits.iter_mut() {
            bm.reset();
        }
        for array in self.freq_mantissa.iter_mut() {
            for bm in array.iter_mut() {
                bm.reset_flat();
            }
        }
        for con in self.contexts.iter_mut() {
            con.reset();
        }
        self.last_log_token = 1;
    }

    fn encode<W: io::Write>(&mut self, mut dist: super::Distance, ctx: &super::Context, eh: &mut ari::Encoder<W>) {
        dist += 1;
        let log = Model::isize_log(dist);
        let context = &mut self.contexts[ctx.symbol as usize];
        let avg_log = Model::isize_log(context.avg_dist as super::Distance);
        let avg_log_capped = cmp::min(MAX_LOG_CONTEXT, avg_log);
        // write exponent
        {   // base part
            let sym_freq = &mut context.freq_log;
            let log_capped = cmp::min(log, MAX_LOG_CODE)-1;
            let global_freq = &mut self.freq_log[avg_log_capped][self.last_log_token];
            debug!("Dark encoding log {} with context[{}][{}] of sym {}",
                log_capped, avg_log_capped, self.last_log_token, ctx.symbol);
            eh.encode(log_capped, &ari::table::SumProxy::new(1,sym_freq, 2,global_freq, 0)).unwrap();
            sym_freq.update(log_capped, self.update_log_power, self.update_log_add);
            global_freq.update(log_capped, self.update_log_global, self.update_log_add);
        }
        if log >= MAX_LOG_CODE {    // extension
            let freq_log_bits = &mut self.freq_log_bits[if avg_log_capped==MAX_LOG_CONTEXT {1} else {0}];
            for i in MAX_LOG_CODE .. log {
                let bc = &mut context.freq_extra.freqs[i-MAX_LOG_CODE];
                let fc = &mut freq_log_bits.freqs[i-MAX_LOG_CODE];
                eh.encode(true, &ari::bin::SumProxy::new(1,bc, 1,fc, 1)).unwrap();
                bc.update(true);
                fc.update(true);
            }
            let i = log-MAX_LOG_CODE;
            let bc = &mut context.freq_extra.freqs[i];
            let fc = &mut freq_log_bits.freqs[i];
            eh.encode(false, &ari::bin::SumProxy::new(1,bc, 1,fc, 1)).unwrap();
            bc.update(false);
            fc.update(false);
        }
        self.last_log_token = if log<2 {0} else if log<8 {1} else {2};
        // write mantissa
        let mantissa_context = &mut self.freq_mantissa[log];
        for i in 1 .. log {
            let bit = (dist>>(log-i-1)) as usize & 1 != 0;
            if i > MAX_BIT_CONTEXT {
                // just send bits past the model, equally distributed
                eh.encode(bit, mantissa_context.last().unwrap()).unwrap();
            }else {
                let bc = &mut mantissa_context[i-1];
                eh.encode(bit, bc).unwrap();
                bc.update(bit);
            };
        }
        // update the model
        let log_diff = (log as isize) - (avg_log_capped as isize);  //check avg_log
        context.update(dist-1, log_diff);
    }

    fn decode<R: io::Read>(&mut self, ctx: &super::Context, dh: &mut ari::Decoder<R>) -> super::Distance {
        let context = &mut self.contexts[ctx.symbol as usize];
        let avg_log = Model::isize_log(context.avg_dist as super::Distance);
        let avg_log_capped = cmp::min(MAX_LOG_CONTEXT, avg_log);
        // read exponent
        let log_pre = { // base part
            let sym_freq = &mut context.freq_log;
            let global_freq = &mut self.freq_log[avg_log_capped][self.last_log_token];
            let log = dh.decode(&ari::table::SumProxy::new(1, sym_freq, 2, global_freq, 0)).unwrap();
            debug!("Dark decoding log {} with context[{}][{}] of sym {}",
                log, avg_log_capped, self.last_log_token, ctx.symbol);
            sym_freq.update(log, self.update_log_power, self.update_log_add);
            global_freq.update(log, self.update_log_global, self.update_log_add);
            log+1
        };
        let log = if log_pre >= MAX_LOG_CODE {  //extension
            let mut count = 0;
            let freq_log_bits = &mut self.freq_log_bits[if avg_log_capped==MAX_LOG_CONTEXT {1} else {0}];
            loop {
                let bc = &mut context.freq_extra.freqs[count];
                let fc = &mut freq_log_bits.freqs[count];
                let bit = dh.decode(&ari::bin::SumProxy::new(1,bc, 1,fc, 1)).unwrap();
                bc.update(bit);
                fc.update(bit);
                if !bit {break}
                count += 1;
            }
            log_pre + count
        }else {
            log_pre
        };
        self.last_log_token = if log<2 {0} else if log<8 {1} else {2};
        // read mantissa
        let mantissa_context = &mut self.freq_mantissa[log];
        let mut dist = 1 as super::Distance;
        for i in 1 .. log {
            let bit = if i > MAX_BIT_CONTEXT {
                dh.decode( mantissa_context.last().unwrap() ).unwrap()
            }else {
                let bc = &mut mantissa_context[i-1];
                let bit = dh.decode(bc).unwrap();
                bc.update(bit);
                bit
            };
            dist = (dist<<1) + (bit as super::Distance);
        }
        // update model
        let log_diff = (log as isize) - (avg_log_capped as isize);
        dist -= 1;
        context.update(dist, log_diff);
        dist
    }
}
