/*!

Experimental BWT-DC compression model

*/

use std::io;
use compress::entropy::ari;


static FIXED_BASE	: uint = 8;
static FIXED_MASK	: uint = (1<<FIXED_BASE) - 1;
static LOG_LIMIT	: uint = 10;
static LOG_DEFAULT	: uint = 1<<FIXED_BASE;
static BIT_UPDATE	: int = 5;

/// Coding model for BWT-DC output
pub struct Model {
	avg_log	: [uint, ..0x100],	//fixed-point
	prob	: [[ari::apm::Bit, ..24], ..LOG_LIMIT],
}

impl Model {
	/// Create a new model
	pub fn new() -> Model {
		Model {
			avg_log	: [LOG_DEFAULT, ..0x100],
			prob	: [[ari::apm::Bit::new_equal(), ..24], ..LOG_LIMIT],
		}
	}

	fn get_log(d: super::Distance) -> uint {
		let du = d as uint;
		match d {
			0..2	=> d as uint << FIXED_BASE,
			3..4	=> (3 << FIXED_BASE) + (du&1)*(FIXED_BASE>>1),
			5..7	=> (4 << FIXED_BASE) + ((du-5)%3)*(FIXED_BASE/3),
			8..12	=> (5 << FIXED_BASE) + (du&3)*(FIXED_BASE>>2),
			_		=> (6 << FIXED_BASE) + (du-12)*(FIXED_BASE>>12),
		}
	}
}

impl super::DistanceModel for Model {
	fn new_default() -> Model {
		Model::new()
	}

	fn reset(&mut self) {
		for log in self.avg_log.mut_iter() {
			*log = LOG_DEFAULT;
		}
		for log in self.prob.mut_iter() {
			for bit in log.mut_iter() {
				*bit = ari::apm::Bit::new_equal();
			}
		}
	}

	fn encode<W: io::Writer>(&mut self, dist: super::Distance, ctx: &super::Context, eh: &mut ari::Encoder<W>) {
		// find context
		let log = self.avg_log[ctx.symbol as uint];
		let w2 = (log & FIXED_MASK) as uint;
		let w1 = FIXED_MASK + 1 - w2;
		let (pr1,pr2) = self.prob.mut_split_at((log>>FIXED_BASE)+1);
		let (m1,m2) = (pr1.mut_last().unwrap(), &mut pr2[0]);
		// encode
		for (i,(b1,b2)) in m1.mut_iter().zip(m2.mut_iter()).enumerate().rev() {
			let value = dist & (1<<i) != 0;
			let flat = (w1*(b1.to_flat() as uint) + w2*(b2.to_flat() as uint)) >> FIXED_BASE;
			let bit = ari::apm::Bit::from_flat(flat as ari::apm::FlatProbability);
			eh.encode(value, &bit).unwrap();
			b1.update(value, BIT_UPDATE, 0);
			b2.update(value, BIT_UPDATE, 0);
		}
		// update
		self.avg_log[ctx.symbol as uint] = (3*log + Model::get_log(dist)) >> 2;
	}

	fn decode<R: io::Reader>(&mut self, ctx: &super::Context, dh: &mut ari::Decoder<R>) -> super::Distance {
		// find context
		let log = self.avg_log[ctx.symbol as uint];
		let w2 = (log & FIXED_MASK) as uint;
		let w1 = FIXED_MASK + 1 - w2;
		let (pr1,pr2) = self.prob.mut_split_at((log>>FIXED_BASE)+1);
		let (m1,m2) = (pr1.mut_last().unwrap(), &mut pr2[0]);
		// decode
		let dist = m1.mut_iter().zip(m2.mut_iter()).rev().fold(0 as super::Distance, |u,(b1,b2)| {
			let flat = (w1*(b1.to_flat() as uint) + w2*(b2.to_flat() as uint)) >> FIXED_BASE;
			let bit = ari::apm::Bit::from_flat(flat as ari::apm::FlatProbability);
			let value = dh.decode(&bit).unwrap();
			b1.update(value, BIT_UPDATE, 0);
			b2.update(value, BIT_UPDATE, 0);
			(u<<1) + if value {1} else {0}
		});
		// update
		self.avg_log[ctx.symbol as uint] = (3*log + Model::get_log(dist)) >> 2;
		dist
	}
}
