#![crate_id = "cluster"]
#![crate_type = "bin"]

//! Cluster analysis tool for DC model raw dump

use std::num::Float;
use std::vec::Vec;

mod brute;
mod cell;


#[deriving(Clone)]
struct Value {
	distance	: uint,
	dist_log	: f32,
	// primary context
	symbol		: uint,
	last_rank	: uint,
	dist_limit	: uint,
	// secondary global context
	rank_avg	: uint,
	dist_lim_log: f32,
	dist_log_avg: f32,
}


struct ReadContext {
	rank_avg	: u8,
	dist_log_avg: f32,
}

impl ReadContext {
	fn new() -> ReadContext {
		ReadContext {
			rank_avg		: 0,
			dist_log_avg	: 0.0,
		}
	}

	fn get_log(d: u32) -> f32 {
		((d+1) as f32).log2()
	}

	fn read<R: Reader>(&mut self, rd: &mut R) -> std::io::IoResult<Value> {
		let d = try!(rd.read_le_u32());
		let d_log = ReadContext::get_log(d);
		let sym = try!(rd.read_u8());
		let rank = try!(rd.read_u8());
		let lim = try!(rd.read_le_u32());
		self.rank_avg = (3*self.rank_avg + rank)>>2;
		self.dist_log_avg = (2.0*self.dist_log_avg + d_log) / 3.0;
		Ok(Value{
			distance	: d as uint,
			dist_log	: d_log,
			symbol		: sym as uint,
			last_rank	: rank as uint,
			dist_limit	: lim as uint,
			rank_avg	: self.rank_avg as uint,
			dist_lim_log: ReadContext::get_log(lim),
			dist_log_avg: self.dist_log_avg,
		})
	}
}


fn process_print(values: Vec<Value>) {
	let mut output = std::io::BufferedWriter::new(
		std::io::File::create(&std::path::Path::new("dump.csv"))
		);
	for v in values.iter() {
		let s = format!("{}, {}, {}, {}\n", v.distance,
			v.symbol, v.last_rank, v.dist_limit);
		output.write_str(s).unwrap();
	}
}


fn main() {
	println!("Cluster analysis tool for Dark context research");
	let args = std::os::args();
	assert!(args.len() > 1);
	let mut input = std::io::BufferedReader::new(
		std::io::File::open(&std::path::Path::new(
			args[1].clone()
			 )));
	let dump_numbers: ~[uint] = args.slice_from(2).iter().map(|s| from_str(*s).unwrap()).collect();
	let mut values: Vec<Value> = Vec::new();
	// read stuff
	let mut rcon = ReadContext::new();
	loop {
		let v = match rcon.read(&mut input) {
			Ok(v)	=> v,
			Err(_e)	=> break,
		};
		values.push(v);
	}

	println!("Got {} values from {}", values.len(), args[1]);
	if false {
		brute::process(values, dump_numbers);
	}else if true {
		cell::process(values, dump_numbers);
	}else {
		process_print(values);
	}
	println!("Done.");
}
