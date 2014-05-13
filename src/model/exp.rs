/*!

Experimental BWT-DC compression model

*/

use std::{cmp, io};
use compress::entropy::ari;


/// Coding model for BWT-DC output
pub struct Model {
	bin_zero	: ari::bin::Model,
	table_log	: ari::table::Model,
	bin_rest	: [ari::bin::Model, ..4],
}

impl Model {
	/// Create a new model with a given max probability threshold
	pub fn new(threshold: ari::Border) -> Model {
		let num_logs = 24;
		Model {
			bin_zero	: ari::bin::Model::new_flat(threshold),
			table_log	: ari::table::Model::new_custom(num_logs, threshold, |i| {
				1<<(10 - cmp::min(10,i))
			}),
			bin_rest	: [ari::bin::Model::new_flat(threshold), ..4],
		}
	}
}

impl super::DistanceModel for Model {
	fn new_default() -> Model {
		Model::new(ari::range_default_threshold >> 2)
	}

	fn reset(&mut self) {
		self.bin_zero.reset_flat();
		self.table_log.reset_flat();
		for bm in self.bin_rest.mut_iter() {
			bm.reset_flat();
		}
	}

	fn encode<W: io::Writer>(&mut self, dist: super::Distance, _ctx: &super::Context, eh: &mut ari::Encoder<W>) {
		let bone = if dist==0 {0} else {1};
		eh.encode(bone, &self.bin_zero).unwrap();
		self.bin_zero.update(bone, 5);
		if bone == 0 {
			return
		}
		fn int_log(d: super::Distance) -> uint {
			let mut log = 0;
			while d>>log !=0 {log += 1;}
			log
		}
		let log = int_log(dist);
		// write exponent
		eh.encode(log-1, &self.table_log).unwrap();
		self.table_log.update(log-1, 10, 1);
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

	fn decode<R: io::Reader>(&mut self, _ctx: &super::Context, dh: &mut ari::Decoder<R>) -> super::Distance {
		let bone = dh.decode(&self.bin_zero).unwrap();
		self.bin_zero.update(bone, 5);
		if bone == 0 {
			return 0
		}
		let log = dh.decode(&self.table_log).unwrap() + 1;
		self.table_log.update(log-1, 10, 1);
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
