/*!

YBS-like compression model

Work in progress, based on the notes from Vadim.

# Links

http://www.compression.ru/ybs/

# Credit

Vadim Yookin for sharing details of YBS implementation.

*/

use std::{cmp, io, num};
use std::vec::Vec;
use compress::entropy::ari;


struct SymbolContext {
	pub avg_log		: uint,
	pub last_diff	: uint,
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
	table_log	: Vec<ari::FrequencyTable>,
	table_high	: ari::FrequencyTable,
	bin_rest	: [ari::BinaryModel, ..3],
	/// specific context tracking
	contexts	: [SymbolContext, ..0x100],
}

impl Model {
	/// Create a new Model instance
	pub fn new(threshold: ari::Border) -> Model {
		let low_logs = 12u;
		Model {
			table_log	: Vec::from_fn(low_logs, |_| ari::FrequencyTable::new_flat(low_logs+1, threshold)),
			table_high	: ari::FrequencyTable::new_flat(32-low_logs, threshold),
			bin_rest	: [ari::BinaryModel::new_flat(threshold), ..3],
			contexts	: [SymbolContext::new(), ..0x100],
		}
	}
}

impl super::DistanceModel for Model {
	fn new_default() -> Model {
		Model::new(ari::range_default_threshold >> 2)
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

	fn encode<W: io::Writer>(&mut self, dist: super::Distance, sym: super::Symbol, eh: &mut ari::Encoder<W>) {
		fn int_log(d: super::Distance) -> uint {
			let mut log = 0;
			while d>>log !=0 {log += 1;}
			log
		}
		let max_low_log = self.table_log.len()-1;
		let log = int_log(dist);
		let context = &mut self.contexts[sym];
		let con_log = cmp::min(context.avg_log, max_low_log);
		let freq_log = self.table_log.get_mut(con_log);
		// write exponent
		let log_encoded = cmp::min(log, max_low_log);
		eh.encode(log_encoded, freq_log).unwrap();
		// update model
		freq_log.update(log_encoded, 10, 1);
		context.update(log_encoded);	//use log?
		if log >= max_low_log {
			let add = log - max_low_log;
			eh.encode(add, &self.table_high).unwrap();
			self.table_high.update(add, 10, 1);
		}
		// write mantissa
		for i in range(1,log) {
			let bit = (dist>>(log-i-1)) as uint & 1;
			if i >= self.bin_rest.len() {
				// just send bits past the model, equally distributed
				eh.encode(bit, self.bin_rest.last().unwrap()).unwrap();
			}else {
				let bc = &mut self.bin_rest[i-1];
				eh.encode(bit, bc).unwrap();
				bc.update(bit, 5);
			};
		}
	}

	fn decode<R: io::Reader>(&mut self, sym: super::Symbol, dh: &mut ari::Decoder<R>) -> super::Distance {
		let max_low_log = self.table_log.len()-1;
		let context = &mut self.contexts[sym];
		let con_log = cmp::min(context.avg_log, max_low_log);
		let freq_log = self.table_log.get_mut(con_log);
		// read exponent
		let log_decoded = dh.decode(freq_log).unwrap();
		// update model
		context.update(log_decoded);
		freq_log.update(log_decoded, 10, 1);
		if log_decoded == 0 {
			return 0
		}
		let log = if log_decoded == max_low_log {
			let add = dh.decode(&self.table_high).unwrap();
			self.table_high.update(add, 10, 1);
			max_low_log + add
		}else {log_decoded};
		// read mantissa
		let mut dist = 1 as super::Distance;
		for i in range(1,log) {
			let bit = if i >= self.bin_rest.len() {
				dh.decode( self.bin_rest.last().unwrap() ).unwrap()
			}else {
				let bc = &mut self.bin_rest[i-1];
				let bit = dh.decode(bc).unwrap();
				bc.update(bit, 5);
				bit
			};
			dist = (dist<<1) + (bit as super::Distance);
		}
		dist
	}
}
