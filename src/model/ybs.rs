/*!
	YBS-like compression model
*/

use std::{cmp, io, num, vec};
use compress::entropy::ari;


struct SymbolContext {
	avg_log		: uint,
	last_diff	: uint,
}

impl SymbolContext {
	fn new() -> SymbolContext {
		SymbolContext{ avg_log:0, last_diff:0 }
	}

	fn update(&mut self, log: uint) {
		let a = if self.last_diff>3 {2u} else {1u};
		let b = 1u;
		self.last_diff = num::abs((log as int) - (self.avg_log as int)) as uint;
		self.avg_log = (a*log + b*self.avg_log) / (a+b);
	}
}


/// Coding model for BWT-DC output
pub struct Model {
	priv freq_log	: ~[ari::FrequencyTable],
	priv freq_rest	: [ari::BinaryModel, ..3],
	priv threshold	: ari::Border,
	/// specific context tracking
	priv contexts	: [SymbolContext, ..0x100],
	/// number of distances processed
	num_processed	: uint,
}

impl Model {
	/// Create a new Model instance
	pub fn new(threshold: ari::Border) -> Model {
		let num_logs = 33u;
		Model {
			freq_log	: vec::from_fn(13, |_| ari::FrequencyTable::new_flat(num_logs, threshold)),
			freq_rest	: [ari::BinaryModel::new_flat(threshold), ..3],
			threshold	: threshold,
			contexts	: [SymbolContext::new(), ..0x100],
			num_processed	: 0,
		}
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
		for bm in self.freq_rest.mut_iter() {
			*bm = ari::BinaryModel::new_flat(self.threshold);
		}
		for con in self.contexts.mut_iter() {
			con.avg_log = 0;
			con.last_diff = 0;
		}
		self.num_processed = 0;
	}

	fn encode<W: io::Writer>(&mut self, dist: super::Distance, sym: super::Symbol, eh: &mut ari::Encoder<W>) {
		fn int_log(d: super::Distance) -> uint {
			let mut log = 0;
			while d>>log !=0 {log += 1;}
			log
		}
		let log = int_log(dist);
		let context = &mut self.contexts[sym];
		let con_log = cmp::min(context.avg_log, self.freq_log.len()-1);
		let freq_log = &mut self.freq_log[con_log];
		// write exponent
		eh.encode(log, freq_log).unwrap();
		// update model
		context.update(log);
		freq_log.update(log, 10, 1);
		self.num_processed += 1;
		// write mantissa
		for i in range(1,log) {
			let bit = (dist>>(log-i-1)) as uint & 1;
			if i > self.freq_rest.len() {
				// just send bits past the model, equally distributed
				eh.encode(bit, self.freq_rest.last().unwrap()).unwrap();
			}else {
				let table = &mut self.freq_rest[i-1];
				eh.encode(bit, table).unwrap();
				table.update(bit, 8, 1);
			};
		}
	}

	fn decode<R: io::Reader>(&mut self, sym: super::Symbol, dh: &mut ari::Decoder<R>) -> super::Distance {
		let context = &mut self.contexts[sym];
		let con_log = cmp::min(context.avg_log, self.freq_log.len()-1);
		let freq_log = &mut self.freq_log[con_log];
		// read exponent
		let log = dh.decode(freq_log).unwrap();
		// update model
		context.update(log);
		freq_log.update(log, 10, 1);
		self.num_processed += 1;
		if log == 0 {
			return 0
		}
		// read mantissa
		let mut dist = 1 as super::Distance;
		for i in range(1,log) {
			let bit = if i > self.freq_rest.len() {
				dh.decode( self.freq_rest.last().unwrap() ).unwrap()
			}else {
				let table = &mut self.freq_rest[i-1];
				let bit = dh.decode(table).unwrap();
				table.update(bit, 8, 1);
				bit
			};
			dist = (dist<<1) + (bit as super::Distance);
		}
		dist
	}
}
