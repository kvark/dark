/*!

Dark-0.51 exact compression model

# Links

http://darchiver.narod.ru/
http://code.google.com/p/adark/

*/

use std::{cmp, io, vec};
use compress::entropy::ari;


/// Aggregate frequency model of two sources
pub struct Aggregate<'a,X,Y> {
	priv x: &'a X,
	priv y: &'a Y,
}

impl<'a, X: ari::Model, Y: ari::Model>
ari::Model for Aggregate<'a,X,Y> {
	fn get_range(&self, value: ari::Value) -> (ari::Border,ari::Border) {
		let (x1,x2) = self.x.get_range(value);
		let (y1,y2) = self.y.get_range(value);
		(x1+y1, x2+y2)
	}

	fn find_value(&self, _offset: ari::Border) -> (ari::Value,ari::Border,ari::Border) {
		(0,0,0)	//TODO
	}

	fn get_denominator(&self) -> ari::Border {
		self.x.get_denominator() + self.y.get_denominator()
	}
}


static MAX_LOG_CONTEXT: uint = 8;
static MAX_BIT_CONTEXT: uint = 3;
static ADAPT_POWERS: [int, ..9] = [6,5,4,3,2,1,4,6,4];
static DIST_OFFSET: super::Distance = 1;


struct SymbolContext {
	avg_dist	: int,
	freq_log	: ari::FrequencyTable,
	freq_extra	: ari::BinaryModel,
}

impl SymbolContext {
	fn new(threshold: ari::Border) -> SymbolContext {
		SymbolContext{
			avg_dist	: 1000,
			freq_log	: ari::FrequencyTable::new_flat(MAX_LOG_CONTEXT+1, threshold),
			freq_extra	: ari::BinaryModel::new_flat(threshold),
		}
	}

	fn reset(&mut self) {
		self.avg_dist = 1000;
		self.freq_log.reset_flat();
		self.freq_extra.reset_flat();
	}

	fn update(&mut self, dist: super::Distance, mut log_diff: int) {
		log_diff = cmp::max(-5, cmp::min(log_diff, 3));
		let adapt = ADAPT_POWERS[5 + log_diff];
		self.avg_dist += (adapt*((dist as int) - self.avg_dist)) >> 3;
	}
}


/// Coding model for BWT-DC output
pub struct Model {
	priv freq_log		: ~[ari::FrequencyTable],
	priv freq_log_bits	: [ari::BinaryModel, ..2],
	priv freq_mantissa	: [ari::BinaryModel, ..MAX_BIT_CONTEXT+1],
	/// specific context tracking
	priv contexts	: ~[SymbolContext],
	/// number of distances processed
	num_processed	: uint,
}

impl Model {
	/// Create a new Model instance
	pub fn new(threshold: ari::Border) -> Model {
		let num_logs = 33u;
		Model {
			freq_log		: vec::from_fn(13, |_| ari::FrequencyTable::new_flat(num_logs, threshold)),
			freq_log_bits	: [ari::BinaryModel::new_flat(threshold), ..2],
			freq_mantissa	: [ari::BinaryModel::new_flat(threshold), ..MAX_BIT_CONTEXT+1],
			contexts		: vec::from_fn(0x100, |_| SymbolContext::new(threshold)),
			num_processed	: 0,
		}
	}

	fn int_log(d: super::Distance) -> uint {
		let mut log = 0;
		while d>>log !=0 {log += 1;}
		log
	}
}

impl super::DistanceModel for Model {
	fn new_default() -> Model {
		Model::new(ari::range_default_threshold >> 2)
	}

	fn reset(&mut self) {
		for table in self.freq_log.mut_iter() {
			table.reset_flat();
		}
		for bm in self.freq_log_bits.mut_iter() {
			bm.reset_flat();
		}
		for bm in self.freq_mantissa.mut_iter() {
			bm.reset_flat();
		}
		for con in self.contexts.mut_iter() {
			con.reset();
		}
		self.num_processed = 0;
	}

	fn encode<W: io::Writer>(&mut self, mut dist: super::Distance, sym: super::Symbol, eh: &mut ari::Encoder<W>) {
		dist = dist + DIST_OFFSET;
		let log = Model::int_log(dist);
		let context = &mut self.contexts[sym];
		let avg_log = Model::int_log(context.avg_dist as super::Distance);
		let avg_log_capped = cmp::min(11, avg_log);
		let log_diff = (log as int) - (avg_log_capped as int);	//check avg_log
		// write exponent & update
		let freq_log_bits = &mut self.freq_log_bits[if avg_log_capped==11 {1} else {0}];
		if log >= MAX_LOG_CONTEXT {
			let sym_freq = &mut context.freq_log;
			eh.encode(MAX_LOG_CONTEXT, sym_freq).unwrap();
			sym_freq.update(MAX_LOG_CONTEXT, 5, 5);
			for _ in range(MAX_LOG_CONTEXT,log) {
				let bc = &mut context.freq_extra;
				eh.encode(0, &ari::BinarySumProxy::new(bc,freq_log_bits)).unwrap();
				bc.update(0, 3);
				freq_log_bits.update(0, 2);
			}
			let bc = &mut context.freq_extra;
			eh.encode(1, &ari::BinarySumProxy::new(bc,freq_log_bits)).unwrap();
			bc.update(1, 3);
			freq_log_bits.update(1, 2);
		}else {
			let sym_freq = &mut context.freq_log;
			eh.encode(log, sym_freq).unwrap();
			sym_freq.update(log, 5, 5);
		}
		// write mantissa
		for i in range(1,log) {
			let bit = (dist>>(log-i-1)) as uint & 1;
			if i > MAX_BIT_CONTEXT {
				// just send bits past the model, equally distributed
				eh.encode(bit, self.freq_mantissa.last().unwrap()).unwrap();
			}else {
				let bc = &mut self.freq_mantissa[i-1];
				eh.encode(bit, bc).unwrap();
				bc.update(bit, 8);
			};
		}
		// update the model
		context.update(dist, log_diff);
		self.num_processed += 1;
	}

	fn decode<R: io::Reader>(&mut self, sym: super::Symbol, dh: &mut ari::Decoder<R>) -> super::Distance {
		let context = &mut self.contexts[sym];
		let avg_log = Model::int_log(context.avg_dist as super::Distance);
		let avg_log_capped = cmp::min(11, avg_log);
		let freq_log_bits = &mut self.freq_log_bits[if avg_log_capped==11 {1} else {0}];
		// read exponent
		let log_pre = {
			let sym_freq = &mut context.freq_log;
			let log = dh.decode(sym_freq).unwrap();
			sym_freq.update(log, 5, 5);
			log
		};
		let log = if log_pre >= MAX_LOG_CONTEXT {
			let mut count = 0;
			let bc = &mut context.freq_extra;
			while dh.decode(&ari::BinarySumProxy::new(bc,freq_log_bits)).unwrap() == 0 {
				count += 1;
				bc.update(0, 3);
				freq_log_bits.update(0, 2);
			}
			bc.update(1, 3);
			freq_log_bits.update(1, 2);
			log_pre + count
		}else {
			log_pre
		};
		// read mantissa
		let mut dist = 1 as super::Distance;
		for i in range(1,log) {
			let bit = if i > MAX_BIT_CONTEXT {
				dh.decode( self.freq_mantissa.last().unwrap() ).unwrap()
			}else {
				let bc = &mut self.freq_mantissa[i-1];
				let bit = dh.decode(bc).unwrap();
				bc.update(bit, 8);
				bit
			};
			dist = (dist<<1) + (bit as super::Distance);
		}
		// update model
		let log_diff = (log as int) - (avg_log as int);
		context.update(dist, log_diff);
		self.num_processed += 1;
		// return
		dist-DIST_OFFSET
	}
}
