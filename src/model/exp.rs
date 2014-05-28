/*!

Experimental BWT-DC compression model

*/

use std::io;
use compress::entropy::ari;


/// Coding model for BWT-DC output
pub struct Model {
	bits	: [ari::apm::Bit, ..24],
}

impl Model {
	/// Create a new model
	pub fn new() -> Model {
		Model {
			bits	: [ari::apm::Bit::new_equal(), ..24],
		}
	}
}

impl super::DistanceModel for Model {
	fn new_default() -> Model {
		Model::new()
	}

	fn reset(&mut self) {
		for bit in self.bits.mut_iter() {
			*bit = ari::apm::Bit::new_equal();
		}
	}

	fn encode<W: io::Writer>(&mut self, dist: super::Distance, _ctx: &super::Context, eh: &mut ari::Encoder<W>) {
		for (i,bit) in self.bits.mut_iter().enumerate().rev() {
			let value = dist & (1<<i) != 0;
			eh.encode(value, bit).unwrap();
			bit.update(value, 5, 0);
		}
	}

	fn decode<R: io::Reader>(&mut self, _ctx: &super::Context, dh: &mut ari::Decoder<R>) -> super::Distance {
		self.bits.mut_iter().rev().fold(0 as super::Distance, |u,bit| {
			let value = dh.decode(bit).unwrap();
			bit.update(value, 5, 0);
			(u<<1) + if value {1} else {0}
		})
	}
}
