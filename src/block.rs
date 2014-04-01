/*!

Block encoding/decoding routines

*/

use std::io;
use std::vec::Vec;
use compress::bwt;
use compress::entropy::ari;
use model::{Distance, DistanceModel};
use saca;


/// A basic block encoder
pub struct Encoder<M> {
	priv sac	: saca::Constructor,
	priv mtf	: bwt::mtf::MTF,
	/// Distance encoding model
	model		: M,
}

#[cfg(tune)]
fn print_stats<W: Writer>(eh: &ari::Encoder<W>) {
	let (b0, b1) = eh.get_bytes_lost();
	info!("Bytes lost on threshold cut: {}, on divisions: {}", b0, b1);
}

#[cfg(not(tune))]
fn print_stats<W: Writer>(_eh: &ari::Encoder<W>) {
	//empty
}

impl<M: DistanceModel> Encoder<M> {
	/// Create a new Encoder instance
	pub fn new(n: uint) -> Encoder<M> {
		Encoder {
			sac		: saca::Constructor::new(n),
			mtf		: bwt::mtf::MTF::new(),
			model	: DistanceModel::new_default(),
		}
	}

	/// Encode a block into a given writer
	pub fn encode<W: Writer>(&mut self, input: &[u8], mut writer: W) -> (W, io::IoResult<()>) {
		let block_size = input.len();
		assert!(block_size <= self.sac.capacity());
		// perform BWT and DC
		let (output, origin) = {
			let suf = self.sac.compute(input);
			let mut iter = bwt::TransformIterator::new(input, suf);
			let out: Vec<u8> = iter.collect();
			(out, iter.get_origin())
		};
		let suf = self.sac.reuse().mut_slice_to(block_size);
		let dc_init = bwt::dc::encode(output.as_slice(), suf, &mut self.mtf);
		// encode alphabet
		let alphabet_size = dc_init.len();
		let mut helper = if alphabet_size > 111 {
			info!("Alphabet is sparse");
			writer.write_u8(0).unwrap();
			let mut rd = [block_size as Distance, ..0x100];
			for &(sym,d) in dc_init.iter() {
				rd[sym] = d;
			}
			let mut eh = ari::Encoder::new(writer);
			for (sym,&d) in rd.iter().enumerate() {
				debug!("Init distance {} for {}", d, sym);
				self.model.encode(d, sym as u8, &mut eh);
			}
			eh
		}else {
			info!("Alphabet of size {}", alphabet_size);
			writer.write_u8(alphabet_size as u8).unwrap();
			for &(s,_) in dc_init.iter() {
				writer.write_u8(s).unwrap();
			}
			let mut eh = ari::Encoder::new(writer);
			for &(sym,d) in dc_init.iter() {
				debug!("Init distance {} for {}", d, sym);
				self.model.encode(d, sym, &mut eh);
			}
			eh
		};
		// encode distances
		for (&d,&sym) in suf.iter().zip(output.iter()) {
			if (d as uint) < block_size {
				debug!("Distance {} for {}", d, sym);
				self.model.encode(d, sym, &mut helper);
			}
		}
		// done
		info!("Origin: {}", origin);
		self.model.encode(origin as Distance, 0, &mut helper);
		print_stats(&helper);
		helper.finish()
	}
}


/// A basic block decoder
pub struct Decoder<M> {
	priv input		: Vec<u8>,
	priv suffixes	: Vec<saca::Suffix>,
	priv mtf		: bwt::mtf::MTF,
	/// Distance decoding model
	model			: M,
}

impl<M: DistanceModel> Decoder<M> {
	/// Create a new decoder instance
	pub fn new(n: uint) -> Decoder<M> {
		Decoder {
			input	: Vec::from_elem(n, 0u8),
			suffixes: Vec::from_elem(n, 0 as saca::Suffix),
			mtf		: bwt::mtf::MTF::new(),
			model	: DistanceModel::new_default(),
		}
	}

	/// Decode a block by reading from a given Reader into some Writer
	pub fn decode<R: Reader, W: Writer>(&mut self, mut reader: R, mut writer: W) -> (R, W, io::IoResult<()>) {
		// decode alphabit
		let alphabet_size = reader.read_u8().unwrap() as uint;
		let mut alphabet = [0u8, ..0x100];
		let alpha_opt = if alphabet_size == 0 {
			info!("Alphabet is sparse");
			None
		}else {
			reader.read( alphabet.mut_slice_to(alphabet_size) ).unwrap();
			info!("Alphabet of size {}: {:?}", alphabet_size, alphabet.slice_to(alphabet_size));
			Some(alphabet.slice_to(alphabet_size))
		};
		// decode distances
		let model = &mut self.model;
		let mut dh = ari::Decoder::new(reader);
		dh.start().unwrap();
		bwt::dc::decode(alpha_opt, self.input.as_mut_slice(), &mut self.mtf, |sym| {
			let d = model.decode(sym, &mut dh);
			debug!("Distance {} for {}", d, sym);
			Ok(d as uint)
		}).unwrap();
		let origin = model.decode(0, &mut dh) as uint;
		info!("Origin: {}", origin);
		// undo BWT and write output
		for b in bwt::decode(self.input.as_slice(), origin, self.suffixes.as_mut_slice()) {
			writer.write_u8(b).unwrap();
		}
		let result = writer.flush();
		(dh.finish(), writer, result)
	}
}


#[cfg(test)]
pub mod test {
	use std::io;
	use std::vec::Vec;
	use test;
	use super::super::model::{DistanceModel, exp, ybs};

	fn roundtrip<M: DistanceModel>(bytes: &[u8]) {
		let (writer, err) = super::Encoder::<M>::new(bytes.len()).encode(bytes, io::MemWriter::new());
		err.unwrap();
		let reader = io::BufReader::new(writer.get_ref());
		let (_, output, err) = super::Decoder::<M>::new(bytes.len()).decode(reader, io::MemWriter::new());
		err.unwrap();
		assert_eq!(bytes.as_slice(), output.get_ref());
	}
	
	#[test]
	fn roundtrips() {
		roundtrip::<exp::Model>(bytes!("abracababra"));
		roundtrip::<exp::Model>	(include_bin!("../lib/compress/data/test.txt"));
		roundtrip::<ybs::Model>	(include_bin!("../lib/compress/data/test.txt"));
	}

	#[bench]
	fn encode_speed(bh: &mut test::BenchHarness) {
		let input = include_bin!("../lib/compress/data/test.txt");
		let mut buffer = Vec::from_elem(input.len(), 0u8);
		let mut encoder = super::Encoder::<ybs::Model>::new(input.len());
		bh.iter(|| {
			let (_, err) = encoder.encode(input, io::BufWriter::new(buffer.as_mut_slice()));
			err.unwrap();
		});
		bh.bytes = input.len() as u64;
	}

	#[bench]
	fn decode_speed(bh: &mut test::BenchHarness) {
		let input = include_bin!("../lib/compress/data/test.txt");
		let mut encoder = super::Encoder::<ybs::Model>::new(input.len());
		encoder.model.reset();
		let (writer, err) = encoder.encode(input, io::MemWriter::new());
		err.unwrap();
		let mut buffer = Vec::from_elem(input.len(), 0u8);
		let mut decoder = super::Decoder::<ybs::Model>::new(input.len());
		bh.iter(|| {
			decoder.model.reset();
			let (_, _, err) = decoder.decode(io::BufReader::new(writer.get_ref()), io::BufWriter::new(buffer.as_mut_slice()));
			err.unwrap();
		});
		bh.bytes = input.len() as u64;
	}
}
