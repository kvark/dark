/*!

Block encoding/decoding routines

*/

use std::{io, vec};
use compress::bwt;
use compress::entropy::ari;
use model;
use saca;


/// A basic block encoder
pub struct Encoder {
	priv sac	: saca::Constructor,
	priv mtf	: bwt::mtf::MTF,
	priv model	: model::dc::Model,
}

impl Encoder {
	/// Create a new Encoder instance
	pub fn new(n: uint) -> Encoder {
		Encoder {
			sac		: saca::Constructor::new(n),
			mtf		: bwt::mtf::MTF::new(),
			model	: model::dc::new(),
		}
	}

	/// Encode a block into a given writer
	pub fn encode<W: Writer>(&mut self, input: &[u8], mut writer: W) -> (W, io::IoResult<()>) {
		let N = input.len();
		assert!(N <= self.sac.capacity());
		// perform BWT and DC
		let (output, origin) = {
			let suf = self.sac.compute(input);
			let mut iter = bwt::TransformIterator::new(input, suf);
			let out = iter.to_owned_vec();
			(out, iter.get_origin())
		};
		let suf = self.sac.reuse().mut_slice_to(N);
		let dc_init = bwt::dc::encode(output, suf, &mut self.mtf);
		// encode alphabet
		let E = dc_init.len();
		let mut helper = if E > 111 {
			info!("Alphabet is sparse");
			writer.write_u8(0).unwrap();
			let mut rd = [N as model::dc::Distance, ..0x100];
			for &(sym,d) in dc_init.iter() {
				rd[sym] = d;
			}
			let mut eh = ari::Encoder::new(writer);
			for (sym,&d) in rd.iter().enumerate() {
				info!("Init distance {} for {}", d, sym);
				self.model.encode(d, sym as u8, &mut eh);
			}
			eh
		}else {
			info!("Alphabet of size {}", E);
			writer.write_u8(E as u8).unwrap();
			writer.write( dc_init.map(|&(s,_)| s) ).unwrap();
			let mut eh = ari::Encoder::new(writer);
			for &(sym,d) in dc_init.iter() {
				info!("Init distance {} for {}", d, sym);
				self.model.encode(d, sym, &mut eh);
			}
			eh
		};
		// encode distances
		for (&d,&sym) in suf.iter().zip(output.iter()) {
			if (d as uint) < N {
				info!("Distance {} for {}", d, sym);
				self.model.encode(d, sym, &mut helper);
			}
		}
		// done
		info!("Origin: {}", origin);
		self.model.encode(origin as model::dc::Distance, 0, &mut helper);
		info!("Encoded {} distances", self.model.num_processed);
		helper.finish()
	}
}


/// A basic block decoder
pub struct Decoder {
	priv input		: ~[u8],
	priv suffixes	: ~[saca::Suffix],
	priv mtf		: bwt::mtf::MTF,
	priv model		: model::dc::Model,
}

impl Decoder {
	/// Create a new decoder instance
	pub fn new(n: uint) -> Decoder {
		Decoder {
			input	: vec::from_elem(n, 0u8),
			suffixes: vec::from_elem(n, 0 as saca::Suffix),
			mtf		: bwt::mtf::MTF::new(),
			model	: model::dc::new(),
		}
	}

	/// Decode a block by reading from a given Reader into some Writer
	pub fn decode<R: Reader, W: Writer>(&mut self, mut reader: R, mut writer: W) -> (R, W, io::IoResult<()>) {
		// decode alphabit
		let E = reader.read_u8().unwrap() as uint;
		let mut alphabet = [0u8, ..0x100];
		let alpha_opt = if E == 0 {
			info!("Alphabet is sparse");
			None
		}else {
			reader.read( alphabet.mut_slice_to(E) ).unwrap();
			info!("Alphabet of size {}: {:?}", E, alphabet.slice_to(E));
			Some(alphabet.slice_to(E))
		};
		// decode distances
		let mut dh = ari::Decoder::new(reader);
		dh.start().unwrap();
		let model = &mut self.model;
		bwt::dc::decode(alpha_opt, self.input, &mut self.mtf, |sym| {
			let d = model.decode(sym, &mut dh);
			info!("Distance {} for {}", d, sym);
			Ok(d as uint)
		}).unwrap();
		let origin = model.decode(0, &mut dh) as uint;
		info!("Origin: {}", origin);
		// undo BWT and write output
		for b in bwt::decode(self.input, origin, self.suffixes) {
			writer.write_u8(b).unwrap();
		}
		let result = writer.flush();
		(dh.finish(), writer, result)
	}
}
