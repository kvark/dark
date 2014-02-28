#[crate_id = "dark"];
#[crate_type = "bin"];
#[deny(warnings, missing_doc)];

//! Dark compressor prototype

extern crate native;
extern crate compress;
#[cfg(test)]
extern crate test;

use std::{io, os};

/// Block encoding/decoding logic
pub mod block;
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
	let file_name = input_path.filename_str().unwrap();
	if file_name.ends_with(extension) {
		let mut in_file = match io::File::open(&input_path) {
			Ok(file) => io::BufferedReader::new(file),
			Err(e) => {
				println!("Input {} can not be read: {}", input_path.as_str(), e.to_str());
				return;
			}
		};
		let ext_pos = file_name.len() - extension.len();
		let out_path = Path::new(format!("{}{}", file_name.slice_to(ext_pos), ".orig"));
		let out_file = io::BufferedWriter::new(io::File::create(&out_path));
		// decode the block size
		let N = in_file.read_le_u32().unwrap() as uint;
		info!("Decoding N: {}", N);
		// decode the block
		let (_, _, err) = block::Decoder::new(N).decode(in_file, out_file);
		err.unwrap();
	}else {
		let input = match io::File::open(&input_path).read_to_end() {
			Ok(data) => data,
			Err(e) => {
				println!("Input {} can not be read: {}", input_path.as_str(), e.to_str());
				return;
			}
		};
		let N = input.len();
		// write the block size
		let out_path = Path::new(format!("{}{}", file_name, ".dark"));
		let mut out_file = io::BufferedWriter::new(io::File::create(&out_path).unwrap());
		info!("Encoding N: {}", N);
		out_file.write_le_u32(N as u32).unwrap();
		// encode the block
		let (_, err) = block::Encoder::new(N).encode(input, out_file);
		err.unwrap();
	}
}
