/*!

A simple coding model to be a baseline for comparison

*/

use std::{cmp, io};
use std::vec::Vec;
use compress::entropy::ari;


/// A pass-though byte frequency model
pub struct Raw;
impl ari::Model<u8> for Raw {
	fn get_range(&self, value: u8) -> (ari::Border, ari::Border) {
		(value as ari::Border, value as ari::Border+1)
	}
	fn find_value(&self, offset: ari::Border) -> (u8, ari::Border, ari::Border) {
		(offset as u8, offset, offset+1)
	}
	fn get_denominator(&self) -> ari::Border {0x100}
}


/// A simple DC model, coding up to 0xFF distances as-is, and with following 3 bytes otherwise
pub struct Model {
	freq: Vec<ari::table::Model>,
	up	: [uint, ..4],
}

impl super::DistanceModel for Model {
	fn new_default() -> Model {
		let threshold = ari::range_default_threshold >> 2;
		Model {
			freq: Vec::from_fn(4, |_| ari::table::Model::new_flat(0x100, threshold)),
			up	: [10,8,7,6],
		}
	}

	fn reset(&mut self) {
		for table in self.freq.mut_iter() {
			table.reset_flat();
		}
	}

	fn encode<W: io::Writer>(&mut self, dist: super::Distance, _ctx: &super::Context, eh: &mut ari::Encoder<W>) {
		let val = cmp::min(0xFF, dist) as uint;
		eh.encode(val, self.freq.get(0)).unwrap();
		self.freq.get_mut(0).update(val, self.up[0], 1);
		if val == 0xFF {
			let rest = (dist - 0xFF) as uint;
			for i in range(0u,3u) {
				let b = (rest>>(i*8))&0xFF;
				eh.encode(b, self.freq.get(i+1)).unwrap();
				self.freq.get_mut(i+1).update(b, self.up[i+1], 1);
			}
		}
	}

	fn decode<R: io::Reader>(&mut self, _ctx: &super::Context, dh: &mut ari::Decoder<R>) -> super::Distance {
		let base = dh.decode(self.freq.get(0)).unwrap();
		self.freq.get_mut(0).update(base, self.up[0], 1);
		let d = if base == 0xFF {
			range(0u,3u).fold(base, |u,i| {
				let b = dh.decode(self.freq.get(i+1)).unwrap();
				self.freq.get_mut(i+1).update(b, self.up[i+1], 1);
				u + (b<<(i*8))
			})
		}else {base};
		d as super::Distance
	}
}
