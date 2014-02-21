#[crate_id = "dark"];
#[crate_type = "bin"];
#[deny(warnings, missing_doc)];

//! Dark compressor prototype


extern crate compress;

use std::{io, os, vec};
use compress::bwt;
use compress::entropy::ari;

/// Suffix Array Construction Algorithm (SACA)
pub mod saca;
/// Compression models
pub mod model {
	/// Distance Coding model
	pub mod dc;
}


/// Program entry point
pub fn main() {
	let extension = &".dark";
	let args = os::args();
	if args.len() <= 1 {
		println!("Dark usage:");
		println!("\t{} input_file[.dark]", args[0]);
		return
	}
	let input_path = Path::new(args[1].clone());
	let mut model = model::dc::new();
	let file_name = input_path.filename_str().unwrap();
	if file_name.ends_with(extension) {
		let mut in_file = match io::File::open(&input_path) {
			Ok(file) => file,
			Err(e) => {
				println!("Input {} can not be read: {}", input_path.as_str(), e.to_str());
				return;
			}
		};
		let N = in_file.read_le_u32().unwrap() as uint;
		info!("Decoding N: {}", N);
		// create temporaries
		let mut input = vec::from_elem(N, 0u8);
		// decode alphabit
		let mut mtf = bwt::mtf::MTF::new();
		let E = in_file.read_u8().unwrap() as uint;
		let mut alphabet = [0u8, ..0x100];
		let alpha_opt = if E == 0 {
			info!("Alphabet is sparse");
			None
		}else {
			in_file.read( alphabet.mut_slice_to(E) ).unwrap();
			info!("Alphabet of size {}: {:?}", E, alphabet.slice_to(E));
			Some(alphabet.slice_to(E))
		};
		// decode distances
		let mut dh = ari::Decoder::new(in_file);
		dh.start().unwrap();
		bwt::dc::decode(alpha_opt, input, &mut mtf, |sym| {
			let d = model.decode(sym, &mut dh);
			info!("Distance {} for {}", d, sym);
			Ok(d as uint)
		}).unwrap();
		let origin = model.decode(0, &mut dh) as uint;
		info!("Origin: {}", origin);
		let mut suf = vec::from_elem(N, N as saca::Suffix);
		// undo BWT and write output
		let ext_pos = file_name.len() - extension.len();
		let out_path = Path::new(format!("{}{}", file_name.slice_to(ext_pos), ".orig"));
		let mut out_file = io::File::create(&out_path);
		for b in bwt::decode(input, origin, suf) {
			out_file.write_u8(b).unwrap();
		}
	}else {
		let input = match io::File::open(&input_path).read_to_end() {
			Ok(data) => data,
			Err(e) => {
				println!("Input {} can not be read: {}", input_path.as_str(), e.to_str());
				return;
			}
		};
		let N = input.len();
		// create temporary suffix array
		let mut suf = vec::from_elem(N, N as saca::Suffix);
		// do BWT and DC
		let (output, origin) = {
			let mut iter = bwt::encode(input, suf);
			let out = iter.to_owned_vec();
			(out, iter.get_origin())
		};
		let mut mtf = bwt::mtf::MTF::new();
		let dc_init = bwt::dc::encode(output, suf, &mut mtf);
		// compress to the output
		let out_path = Path::new(format!("{}{}", file_name, ".dark"));
		let mut out_file = io::File::create(&out_path).unwrap();
		info!("Encoding N: {}", N);
		out_file.write_le_u32(N as u32).unwrap();
		// encode alphabet
		let E = dc_init.len();
		let mut helper = if E > 111 {
			info!("Alphabet is sparse");
			out_file.write_u8(0).unwrap();
			let mut rd = [N as model::dc::Distance, ..0x100];
			for &(sym,d) in dc_init.iter() {
				rd[sym] = d;
			}
			let mut eh = ari::Encoder::new(out_file);
			for (sym,&d) in rd.iter().enumerate() {
				model.encode(d, sym as u8, &mut eh);
			}
			eh
		}else {
			info!("Alphabet of size {}", E);
			out_file.write_u8(E as u8).unwrap();
			out_file.write( dc_init.map(|&(s,_)| s) ).unwrap();
			let mut eh = ari::Encoder::new(out_file);
			for &(sym,d) in dc_init.iter() {
				info!("Init distance {} for {}", d, sym);
				model.encode(d, sym, &mut eh);
			}
			eh
		};
		// encode distances
		for (&d,&sym) in suf.iter().zip(output.iter()) {
			if (d as uint) < N {
				info!("Distance {} for {}", d, sym);
				model.encode(d, sym, &mut helper);
			}
		}
		// done
		info!("Origin: {}", origin);
		model.encode(origin as model::dc::Distance, 0, &mut helper);
		let (_, err) = helper.finish();
		err.unwrap();
		info!("Encoded {} distances", model.num_processed);
	}
}
