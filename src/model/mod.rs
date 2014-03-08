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

/// Raw (Dist,Sym) pairs output
pub struct Raw;

impl ari::Model for Raw {
	fn get_range(&self, value: ari::Value) -> (ari::Border, ari::Border) {
		(value as ari::Border, value as ari::Border+1)
	}
	fn find_value(&self, offset: ari::Border) -> (ari::Value, ari::Border, ari::Border) {
		(offset as ari::Value, offset, offset+1)
	}
	fn get_denominator(&self) -> ari::Border {
		0x100
	}
}

impl DistanceModel for Raw {
	fn new_default() -> Raw {Raw}
	fn reset(&mut self) {}
	
	fn encode<W: Writer>(&mut self, d: Distance, s: Symbol, enc: &mut ari::Encoder<W>) {
		enc.encode(s as ari::Value, &Raw).unwrap();
		enc.encode(((d>> 0)&0xFF) as ari::Value, &Raw).unwrap();
		enc.encode(((d>> 8)&0xFF) as ari::Value, &Raw).unwrap();
		enc.encode(((d>>16)&0xFF) as ari::Value, &Raw).unwrap();
		enc.encode(((d>>24)&0xFF) as ari::Value, &Raw).unwrap();
	}

	fn decode<R: Reader>(&mut self, s: Symbol, dec: &mut ari::Decoder<R>) -> Distance {
		let sym = dec.decode(&Raw).unwrap() as Symbol;
		assert_eq!(s, sym);
		let d0 = dec.decode(&Raw).unwrap() as Distance;
		let d1 = dec.decode(&Raw).unwrap() as Distance;
		let d2 = dec.decode(&Raw).unwrap() as Distance;
		let d3 = dec.decode(&Raw).unwrap() as Distance;
		d0 + (d1<<8) + (d2<<16) + (d3<<24)
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
	fn roundtrips_raw() {
		roundtrips::<super::Raw>();
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
