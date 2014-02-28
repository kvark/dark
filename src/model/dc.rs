use std::{cmp, io};
use compress::entropy::ari;


pub type Distance = u32;

/// Coding model for BWT-DC output
pub struct Model {
	priv freq_log	: ari::FrequencyTable,
	priv freq_rest	: [ari::BinaryModel, ..4],
	priv threshold	: ari::Border,
	/// number of distances processed
	num_processed	: uint,
}

impl Model {
	/// Create a new model with a given max probability threshold
	pub fn new(threshold: ari::Border) -> Model {
		let num_logs = 33;
		Model {
			freq_log		: ari::FrequencyTable::new_custom(num_logs, threshold, |i| {
				1<<(10 - cmp::min(10,i))
			}),
			freq_rest		: [ari::BinaryModel::new_flat(threshold), ..4],
			threshold		: threshold,
			num_processed	: 0,
		}
	}

	/// Reset the model to a simple redundant state
	pub fn reset(&mut self) {
		self.freq_log.reset_flat();
		for bm in self.freq_rest.mut_iter() {
			*bm = ari::BinaryModel::new_flat(self.threshold);
		}
	}

	/// Encode the distance of a symbol, using the Arithmetic coder
	pub fn encode<W: io::Writer>(&mut self, dist: Distance, _sym: u8, eh: &mut ari::Encoder<W>) {
		fn int_log(d: Distance) -> uint {
			let mut log = 0;
			while d>>log !=0 {log += 1;}
			log
		}
		let log = int_log(dist);
		// write exponent
		self.num_processed += 1;
		eh.encode(log, &self.freq_log).unwrap();
		self.freq_log.update(log, 10, 1);
		// write mantissa
		for i in range(1,log) {
			let bit = (dist>>(log-i-1)) as uint & 1;
			if i >= self.freq_rest.len() {
				// just send bits past the model, equally distributed
				eh.encode(bit, self.freq_rest.last().unwrap()).unwrap();
			}else {
				let table = &mut self.freq_rest[i-1];
				eh.encode(bit, table).unwrap();
				table.update(bit, 8, 1);
			};
		}
	}

	/// Decode the distance of a symbol, using the Arithmetic coder
	pub fn decode<R: io::Reader>(&mut self, _sym: u8, dh: &mut ari::Decoder<R>) -> Distance {
		self.num_processed += 1;
		let log = dh.decode(&self.freq_log).unwrap();
		self.freq_log.update(log, 10, 1);
		if log == 0 {
			return 0
		}
		let mut dist = 1 as Distance;
		for i in range(1,log) {
			let bit = if i >= self.freq_rest.len() {
				dh.decode( self.freq_rest.last().unwrap() ).unwrap()
			}else {
				let table = &mut self.freq_rest[i-1];
				let bit = dh.decode(table).unwrap();
				table.update(bit, 8, 1);
				bit
			};
			dist = (dist<<1) + (bit as Distance);
		}
		dist
	}
}

/// Create a new default model instance
pub fn new() -> Model {
	Model::new(ari::range_default_threshold >> 2)
}
