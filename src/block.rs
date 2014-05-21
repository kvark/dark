/*!

Block encoding/decoding routines

*/

use std::io;
use std::vec::Vec;
use compress::bwt;
use compress::entropy::ari;
use model::{Distance, DistanceModel};
use saca;


static ctx_0: bwt::dc::Context = bwt::dc::Context {
	symbol: 0, last_rank: 0, distance_limit: 0x101,
};

/// A basic block encoder
pub struct Encoder<M> {
	sac	: saca::Constructor,
	mtf	: bwt::mtf::MTF,
	/// Distance encoding model
	pub model	: M,
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
	pub fn encode<W: Writer>(&mut self, input: &[u8], writer: W) -> (W, io::IoResult<()>) {
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
		let mut dc_iter = bwt::dc::encode(output.as_slice(), suf, &mut self.mtf);
		let mut eh = ari::Encoder::new(writer);
		{	// encode init distances
			let mut cur_active = true;
			let mut i = 0u;
			while i<0xFF {
				let base = i;
				if cur_active {
					while i<0xFF && dc_iter.get_init()[i]<block_size {
						i += 1;
					}
					let num = (if base==0 {i} else {i-base-1}) as Distance;
					debug!("Init fill num {}", num);
					self.model.encode(num, &ctx_0, &mut eh);
					for (sym,d) in dc_iter.get_init().iter().enumerate().skip(base).take(i-base) {
						let ctx = bwt::dc::Context::new(sym as u8, 0, input.len());
						self.model.encode(*d as Distance, &ctx, &mut eh);
						debug!("Init {} for {}", *d, sym);
					}
					cur_active = false;
				}else {
					while {i+=1; i<0xFF && dc_iter.get_init()[i]==block_size} {}
					let num = (i-base-1) as Distance;
					debug!("Init empty num {}", num);
					self.model.encode(num, &ctx_0, &mut eh);
					cur_active = true;
				}
			}
		}
		// encode distances
		for (d,ctx) in dc_iter {
			debug!("Distance {} for {}", d, ctx.symbol);
			self.model.encode(d, &ctx, &mut eh);
		}
		// done
		info!("Origin: {}", origin);
		self.model.encode(origin as Distance, &ctx_0, &mut eh);
		print_stats(&eh);
		eh.finish()
	}
}


/// A basic block decoder
pub struct Decoder<M> {
	input		: Vec<u8>,
	suffixes	: Vec<saca::Suffix>,
	mtf			: bwt::mtf::MTF,
	/// Distance decoding model
	pub model	: M,
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
	pub fn decode<R: Reader, W: Writer>(&mut self, reader: R, mut writer: W) -> (R, W, io::IoResult<()>) {
		let model = &mut self.model;
		let mut dh = ari::Decoder::new(reader);
		// decode init distances
		let init = {
			let mut init = [self.input.len(), ..0x100];
			let mut cur_active = true;
			let mut i = 0u;
			while i<0xFF {
				let add  = if i==0 && cur_active {0u} else {1u};
				let num = model.decode(&ctx_0, &mut dh) as uint + add;
				debug!("Init num {}", num);
				if cur_active {
					for (sym,d) in init.mut_iter().enumerate().skip(i).take(num)	{
						let ctx = bwt::dc::Context::new(sym as u8, 0, self.input.len());
						*d = model.decode(&ctx, &mut dh) as uint;
						debug!("Init {} for {}", *d, sym);
					}
					cur_active = false;
				}else {
					cur_active = true;
				}
				i += num;
			}
			init
		};
		// decode distances
		bwt::dc::decode(init, self.input.as_mut_slice(), &mut self.mtf, |ctx| {
			let d = model.decode(&ctx, &mut dh);
			debug!("Distance {} for {}", d, ctx.symbol);
			Ok(d as uint)
		}).unwrap();
		let origin = model.decode(&ctx_0, &mut dh) as uint;
		info!("Origin: {}", origin);
		// undo BWT and write output
		for b in bwt::decode(self.input.as_slice(), origin, self.suffixes.as_mut_slice()) {
			writer.write_u8(b).unwrap();
		}
		let result = writer.flush();
		let (r, err) = dh.finish();
		(r, writer, result.and(err))
	}
}


#[cfg(test)]
pub mod test {
	use std::io;
	use std::vec::Vec;
	use test::Bencher;
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
	fn encode_speed(bh: &mut Bencher) {
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
	fn decode_speed(bh: &mut Bencher) {
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
