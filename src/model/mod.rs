/*!

Various BWT-DC compression models

*/

use compress::entropy::ari;
use std::io; //TEMP

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


/// Raw (Sym,Dist) pairs output
pub struct RawOut {
	priv out: io::File,
}

impl DistanceModel for RawOut {
	fn new_default() -> RawOut {
		RawOut {
			out: io::File::create(&Path::new("out.raw")).unwrap(),
		}
	}

	fn reset(&mut self) {}

	fn encode<W: Writer>(&mut self, d: Distance, s: Symbol, _enc: &mut ari::Encoder<W>) {
		debug!("Encoding raw distance {} for symbol {}", d, s);
		self.out.write_u8(s).unwrap();
		self.out.write_le_u32(d).unwrap();
	}

	fn decode<R: Reader>(&mut self, _s: Symbol, _dec: &mut ari::Decoder<R>) -> Distance {
		0	//not supported
	}
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
			debug!("Encode: {}", dist);
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
			debug!("Actual: {}, Decoded: {}", dist, d2);
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

	fn roundtrips<M: DistanceModel>() {
		roundtrip::<M>([(1,1),(2,2),(3,3),(4,4)]);
		roundtrip::<M>(gen_data(1000,200));
	}
	
	#[test]
	fn roundtrips_dark() {
		roundtrips::<super::dark::Model>();
	}

	#[test]
	fn roundtrips_exp() {
		roundtrips::<super::exp::Model>();
	}

	#[test]
	fn roundtrips_ybs() {
		roundtrips::<super::ybs::Model>();
	}
}
