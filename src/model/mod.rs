/*!

Various BWT-DC compression models

*/

use compress::entropy::ari;

/// Old Dark-0.51 model
pub mod dark;
/// Original BWT-DC compression model
pub mod exp;
/// A attempt to reproduce YBS model
pub mod ybs;

pub type Distance = u32;
pub type Symbol = u8;


/// A generic BWT-DC output coding model
pub trait DistanceModel {
	/// Create a new default instance
	fn new_default() -> Self;
	/// Reset current estimations
	fn reset(&mut self);
	/// Encode a distance for some symbol
	fn encode<W: Writer>(&mut self, Distance, Symbol, &mut ari::Encoder<W>);
	/// Decode a distance for some symbol
	fn decode<R: Reader>(&mut self, Symbol, &mut ari::Decoder<R>) -> Distance;
}


#[cfg(test)]
pub mod test {
	use std::{io, rand, vec};
	use compress::entropy::ari;
	use super::{Distance, DistanceModel};
	use super::Symbol;

	fn roundtrip<M: DistanceModel>(input: &[(Symbol,Distance)]) {
		let mut m: M = DistanceModel::new_default();
		let mut eh = ari::Encoder::new(io::MemWriter::new());
		m.reset();
		for &(sym,dist) in input.iter() {
			println!("Encode: {}", dist);
			m.encode(dist, sym, &mut eh);
		}
		let (mem, err) = eh.finish();
		err.unwrap();
		let buffer = mem.unwrap();
		m.reset();
		let mut dh = ari::Decoder::new(io::BufReader::new(buffer));
		dh.start().unwrap();
		for &(sym,dist) in input.iter() {
			let d2 = m.decode(sym, &mut dh);
			println!("Actual: {}, Decoded: {}", dist, d2);
			assert_eq!(d2, dist);
		}
	}

	fn gen_data(size: uint, max_dist: Distance) -> ~[(Symbol,Distance)] {
		use std::rand::Rng;
		let mut rng = rand::rng();
		vec::from_fn(size, |_| {
			(rng.gen::<Symbol>(), rng.gen_range(0, max_dist))
		})
	}
	
	#[test]
	fn roundtrips_dark() {
		roundtrip::<super::dark::Model>([(1,1),(2,2),(3,3),(4,4)]);
		roundtrip::<super::dark::Model>(gen_data(1000,200));
	}

	#[test]
	fn roundtrips_exp() {
		roundtrip::<super::exp::Model>([(1,1),(2,2),(3,3),(4,4)]);
		roundtrip::<super::exp::Model>(gen_data(1000,200));
	}

	#[test]
	fn roundtrips_ybs() {
		roundtrip::<super::ybs::Model>([(1,1),(2,2),(3,3),(4,4)]);
		roundtrip::<super::ybs::Model>(gen_data(1000,200));
	}
}
