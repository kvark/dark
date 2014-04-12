#![crate_id = "dark"]
#![crate_type = "bin"]
#![deny(warnings, missing_doc)]
#![feature(phase)]

//! Dark compressor prototype

#[phase(syntax, link)]
extern crate log;
extern crate getopts;
extern crate compress;
#[cfg(test)]
extern crate rand;
#[cfg(test)]
extern crate test;

use std::{io};
use std::vec::Vec;
use model::DistanceModel;

/// Block encoding/decoding logic
pub mod block;
/// Compression models
pub mod model;
/// Suffix Array Construction Algorithm (SACA)
pub mod saca;


/// Program entry point
pub fn main() {
	let extension = &".dark";
	let options = [
		getopts::optopt("m", "model", "set compression model", "dark|exp|raw|simple|ybs"),
		//getopts::optopt("o", "output", "set output file name", "NAME"),
		getopts::optflag("h", "help", "print this help info"),
	];
	//let args = os::args();
	let mut args: Vec<~str> = Vec::new();
	args.push(~"dark");
	args.push(~"-m");
	args.push(~"dark");
	args.push(~"data/book1");
	let matches = match getopts::getopts(args.tail(), options) {
		Ok(m)	=> m,
		Err(f)	=> fail!(f.to_err_msg())
	};
	if matches.opt_present("h") || matches.free.is_empty() {
		println!("{}", getopts::usage(
			format!("Dark compressor usage:\n{} [options] input_file[.dark]", *args.get(0)),
			options));
		return
	}

	let model = matches.opt_str("m").unwrap_or(~"exp");
	info!("Using model: {}", model);
	let input_path = Path::new(matches.free.get(0).clone());
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
		let n = in_file.read_le_u32().unwrap() as uint;
		info!("Decoding N: {}", n);
		// decode the block
		let (_, _, err) = match model.as_slice() {
			"dark"	=> block::Decoder::<model::dark::Model>		::new(n).decode(in_file, out_file),
			"exp"	=> block::Decoder::<model::exp::Model>		::new(n).decode(in_file, out_file),
			"raw"	=> block::Decoder::<model::RawOut>			::new(n).decode(in_file, out_file),
			"simple"=> block::Decoder::<model::simple::Model>	::new(n).decode(in_file, out_file),
			"ybs"	=> block::Decoder::<model::ybs::Model>		::new(n).decode(in_file, out_file),
			_		=> fail!("Unknown decoding model: {}", model)
		};
		err.unwrap();
	}else {
		let input = match io::File::open(&input_path).read_to_end() {
			Ok(data) => data,
			Err(e) => {
				println!("Input {} can not be read: {}", input_path.as_str(), e.to_str());
				return;
			}
		};
		let n = input.len();
		// write the block size
		let out_path = Path::new(format!("{}{}", file_name, ".dark"));
		let mut out_file = io::BufferedWriter::new(io::File::create(&out_path).unwrap());
		info!("Encoding N: {}", n);
		out_file.write_le_u32(n as u32).unwrap();
		// encode the block
		let (_, err) = match model.as_slice() {
			"dark"	=> block::Encoder::<model::dark::Model>		::new(n).encode(input.as_slice(), out_file),
			"exp"	=> block::Encoder::<model::exp::Model>		::new(n).encode(input.as_slice(), out_file),
			"raw"	=> block::Encoder::<model::RawOut>			::new(n).encode(input.as_slice(), out_file),
			"simple"=> block::Encoder::<model::simple::Model>	::new(n).encode(input.as_slice(), out_file),
			"ybs"	=> block::Encoder::<model::ybs::Model>		::new(n).encode(input.as_slice(), out_file),
			_		=> fail!("Unknown encoding model: {}", model)
		};
		err.unwrap();
	}
}
