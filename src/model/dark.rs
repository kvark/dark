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


static MAX_LOG_CODE		: uint = 8;
static MAX_LOG_CONTEXT	: uint = 11;
static NUM_LAST_LOGS	: uint = 3;
static MAX_BIT_CONTEXT	: uint = 3;
static ADAPT_POWERS: [int, ..9] = [6,5,4,3,2,1,4,6,4];
static DIST_OFFSET: super::Distance = 1;


struct BinaryMultiplex {
	freqs: [ari::BinaryModel, ..32],
}

impl BinaryMultiplex {
	fn new(threshold: ari::Border) -> BinaryMultiplex {
		BinaryMultiplex {
			freqs: [ari::BinaryModel::new_flat(threshold), ..32],
		}
	}
	fn reset(&mut self) {
		for fr in self.freqs.mut_iter() {
			fr.reset_flat();
		}
	}
}


struct SymbolContext {
	avg_dist	: int,
	freq_log	: ari::FrequencyTable,
	freq_extra	: BinaryMultiplex,
}

impl SymbolContext {
	fn new(threshold: ari::Border) -> SymbolContext {
		SymbolContext{
			avg_dist	: 1000,
			freq_log	: ari::FrequencyTable::new_flat(MAX_LOG_CODE+1, threshold),
			freq_extra	: BinaryMultiplex::new(threshold),
		}
	}

	fn reset(&mut self) {
		self.avg_dist = 1000;
		self.freq_log.reset_flat();
		self.freq_extra.reset();
	}

	fn update(&mut self, dist: super::Distance, mut log_diff: int) {
		log_diff = cmp::max(-6, cmp::min(log_diff, 2));
		let adapt = ADAPT_POWERS[6 + log_diff];
		self.avg_dist += (adapt*((dist as int) - self.avg_dist)) >> 3;
	}
}


/// Coding model for BWT-DC output
pub struct Model {
	priv freq_log		: ~[~[ari::FrequencyTable]],	//[MAX_LOG_CONTEXT+1][NUM_LAST_LOGS]
	priv freq_log_bits	: [BinaryMultiplex, ..2],
	priv freq_mantissa	: [[ari::BinaryModel, ..MAX_BIT_CONTEXT+1], ..32],
	/// specific context tracking
	priv contexts		: ~[SymbolContext],
	priv last_log_token	: uint,
	/// update parameters
	priv update_log_global		: uint,
	priv update_log_power		: uint,
	priv update_log_add			: ari::Border,
	priv update_bits_global		: uint,
	priv update_bits_sym		: uint,
	priv update_mantissa_global	: uint,
}

impl Model {
	/// Create a new Model instance
	pub fn new(threshold: ari::Border) -> Model {
		Model {
			freq_log		: vec::from_fn(MAX_LOG_CONTEXT+1, |_| {
				vec::from_fn(NUM_LAST_LOGS, |_|
					ari::FrequencyTable::new_flat(MAX_LOG_CODE+1, threshold))
			}),
			freq_log_bits	: [BinaryMultiplex::new(threshold), ..2],
			freq_mantissa	: [[ari::BinaryModel::new_flat(threshold), ..MAX_BIT_CONTEXT+1], ..32],
			contexts		: vec::from_fn(0x100, |_| SymbolContext::new(threshold)),
			last_log_token	: 0,
			update_log_global		: 12,
			update_log_power		: 5,
			update_log_add			: 5,
			update_bits_global		: 2,
			update_bits_sym			: 3,
			update_mantissa_global	: 8,
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
		for array in self.freq_log.mut_iter() {
			for table in array.mut_iter() {
				table.reset_flat();
			}
		}
		for bm in self.freq_log_bits.mut_iter() {
			bm.reset();
		}
		for array in self.freq_mantissa.mut_iter() {
			for bm in array.mut_iter() {
				bm.reset_flat();
			}
		}
		for con in self.contexts.mut_iter() {
			con.reset();
		}
		self.last_log_token = 0;
	}

	fn encode<W: io::Writer>(&mut self, mut dist: super::Distance, sym: super::Symbol, eh: &mut ari::Encoder<W>) {
		dist = dist + DIST_OFFSET;
		let log = Model::int_log(dist);
		let context = &mut self.contexts[sym];
		let avg_log = Model::int_log(context.avg_dist as super::Distance);
		let avg_log_capped = cmp::min(MAX_LOG_CONTEXT, avg_log);
		// write exponent
		{	// base part
			let sym_freq = &mut context.freq_log;
			let log_capped = cmp::min(log, MAX_LOG_CODE);
			let global_freq = &mut self.freq_log[avg_log_capped][self.last_log_token];
			debug!("Dark encoding log {} with context[{}][{}] of sym {}",
				log_capped, avg_log_capped, self.last_log_token, sym);
			eh.encode(log_capped, &ari::TableSumProxy::new(sym_freq, global_freq)).unwrap();
			sym_freq.update(log_capped, self.update_log_power, self.update_log_add);
			global_freq.update(log_capped, self.update_log_global, self.update_log_add);
		}
		if log >= MAX_LOG_CODE {	// extension
			let freq_log_bits = &mut self.freq_log_bits[if avg_log_capped==MAX_LOG_CONTEXT {1} else {0}];
			for i in range(MAX_LOG_CODE, log) {
				let bc = &mut context.freq_extra.freqs[i-MAX_LOG_CODE];
				let fc = &mut freq_log_bits.freqs[i-MAX_LOG_CODE];
				eh.encode(0, &ari::BinarySumProxy::new(bc, fc)).unwrap();
				bc.update(0, self.update_bits_sym);
				fc.update(0, self.update_bits_global);
			}
			let i = log-MAX_LOG_CODE;
			let bc = &mut context.freq_extra.freqs[i];
			let fc = &mut freq_log_bits.freqs[i];
			eh.encode(1, &ari::BinarySumProxy::new(bc, fc)).unwrap();
			bc.update(1, self.update_bits_sym);
			fc.update(1, self.update_bits_global);
		}
		self.last_log_token = if log<2 {0} else if log<8 {1} else {2};
		// write mantissa
		let mantissa_context = &mut self.freq_mantissa[log];
		for i in range(1,log) {
			let bit = (dist>>(log-i-1)) as uint & 1;
			if i > MAX_BIT_CONTEXT {
				// just send bits past the model, equally distributed
				eh.encode(bit, mantissa_context.last().unwrap()).unwrap();
			}else {
				let bc = &mut mantissa_context[i-1];
				eh.encode(bit, bc).unwrap();
				bc.update(bit, self.update_mantissa_global);
			};
		}
		// update the model
		let log_diff = (log as int) - (avg_log_capped as int);	//check avg_log
		context.update(dist, log_diff);
	}

	fn decode<R: io::Reader>(&mut self, sym: super::Symbol, dh: &mut ari::Decoder<R>) -> super::Distance {
		let context = &mut self.contexts[sym];
		let avg_log = Model::int_log(context.avg_dist as super::Distance);
		let avg_log_capped = cmp::min(MAX_LOG_CONTEXT, avg_log);
		// read exponent
		let log_pre = { // base part
			let sym_freq = &mut context.freq_log;
			let global_freq = &mut self.freq_log[avg_log_capped][self.last_log_token];
			let log = dh.decode(&ari::TableSumProxy::new(sym_freq, global_freq)).unwrap();
			debug!("Dark decoding log {} with context[{}][{}] of sym {}",
				log, avg_log_capped, self.last_log_token, sym);
			sym_freq.update(log, self.update_log_power, self.update_log_add);
			global_freq.update(log, self.update_log_global, self.update_log_add);
			log
		};
		let log = if log_pre >= MAX_LOG_CODE {	//extension
			let mut count = 0;
			let freq_log_bits = &mut self.freq_log_bits[if avg_log_capped==MAX_LOG_CONTEXT {1} else {0}];
			loop {
				let bc = &mut context.freq_extra.freqs[count];
				let fc = &mut freq_log_bits.freqs[count];
				let bit = dh.decode(&ari::BinarySumProxy::new(bc, fc)).unwrap();
				bc.update(bit, self.update_bits_sym);
				fc.update(bit, self.update_bits_global);
				if bit == 1 {break}
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
		for i in range(1,log) {
			let bit = if i > MAX_BIT_CONTEXT {
				dh.decode( mantissa_context.last().unwrap() ).unwrap()
			}else {
				let bc = &mut mantissa_context[i-1];
				let bit = dh.decode(bc).unwrap();
				bc.update(bit, self.update_mantissa_global);
				bit
			};
			dist = (dist<<1) + (bit as super::Distance);
		}
		// update model
		let log_diff = (log as int) - (avg_log_capped as int);
		context.update(dist, log_diff);
		// return
		dist-DIST_OFFSET
	}
}
