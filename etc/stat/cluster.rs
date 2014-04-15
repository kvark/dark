#![crate_id = "cluster"]
#![crate_type = "bin"]
#![deny(warnings, missing_doc)]

//! Cluster analysis tool for DC model raw dump

use std::io;
use std::num::Float;
use std::vec::Vec;


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

impl Value {
	fn get_distance(&self, other: &Value) -> f32 {
		(self.dist_log - other.dist_log).abs() +
		(if self.symbol == other.symbol {0f32} else {1f32}) +
		(if self.rank_avg == other.rank_avg {0f32} else {0.4f32}) +
		(self.dist_lim_log - other.dist_lim_log).abs() * 0.2f32 + 
		(self.dist_log_avg - other.dist_log_avg).abs()
	}

	fn add(&self, other: &Value) -> Value {
		Value {
			distance	: self.distance		+ other.distance,
			dist_log	: self.dist_log		+ other.dist_log,
			symbol		: self.symbol		+ other.symbol,
			last_rank	: self.last_rank	+ other.last_rank,
			dist_limit	: self.dist_limit	+ other.dist_limit,
			rank_avg	: self.rank_avg		+ other.rank_avg,
			dist_lim_log: self.dist_lim_log + other.dist_lim_log,
			dist_log_avg: self.dist_log_avg	+ other.dist_log_avg,
		}
	}

	fn sub(&self, other: &Value) -> Value {
		Value {
			distance	: self.distance		- other.distance,
			dist_log	: self.dist_log		- other.dist_log,
			symbol		: self.symbol		- other.symbol,
			last_rank	: self.last_rank	- other.last_rank,
			dist_limit	: self.dist_limit	- other.dist_limit,
			rank_avg	: self.rank_avg		- other.rank_avg,
			dist_lim_log: self.dist_lim_log - other.dist_lim_log,
			dist_log_avg: self.dist_log_avg	- other.dist_log_avg,
		}
	}

	fn div(&self, num: uint) -> Value {
		Value {
			distance	: self.distance		/ num,
			dist_log	: self.dist_log		/ (num as f32),
			symbol		: self.symbol		/ num,
			last_rank	: self.last_rank	/ num,
			dist_limit	: self.dist_limit	/ num,
			rank_avg	: self.rank_avg		/ num,
			dist_lim_log: self.dist_lim_log / (num as f32),
			dist_log_avg: self.dist_log_avg	/ (num as f32),
		}
	}

	fn sqr(&self) -> Value {
		Value {
			distance	: self.distance		* self.distance,
			dist_log	: self.dist_log		* self.dist_log,
			symbol		: self.symbol		* self.symbol,
			last_rank	: self.last_rank	* self.last_rank,
			dist_limit	: self.dist_limit	* self.dist_limit,
			rank_avg	: self.rank_avg		* self.rank_avg,
			dist_lim_log: self.dist_lim_log * self.dist_lim_log,
			dist_log_avg: self.dist_log_avg	* self.dist_log_avg,
		}
	}
}

struct Element {
	m0		: uint,		// count
	m1		: Value,	// sum
	m2		: Value,	// sum of squares
	active	: Value,	// mean value
}

impl Element {
	fn new_merge(a: &Element, b: &Element) -> Element {
		let count = a.m0 + b.m0;
		let sum = a.m1.add(&b.m1);
		Element {
			m0	: count,
			m1	: sum,
			m2	: a.m2.add(&b.m2),
			active	: sum.div(count),
		}
	}
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



fn main() {
	println!("Cluster analysis tool for Dark context research");
	let args = std::os::args();
	assert!(args.len() > 1);
	let mut input = std::io::BufferedReader::new(
		std::io::File::open(&std::path::Path::new(
			args[1].clone()
			 )));
	let dump_numbers: ~[uint] = args.slice_from(2).iter().map(|s| from_str(*s).unwrap()).collect();
	let mut dump_id = 0u;
	let mut elements: Vec<Element> = Vec::new();
	// read stuff
	let mut rcon = ReadContext::new();
	loop {
		match rcon.read(&mut input) {
			Ok(value)	=> {
				let sqr = value.sqr();
				elements.push(Element {
					m0	: 1,
					m1	: value.clone(),
					m2	: sqr,
					active	: value,
				});
			},
			Err(_e)	=> break,
		}
	}
	println!("Got {} elements from {}", elements.len(), args[1]);
	// process
	while elements.len()>1 {
		let mut best_id = (0u,0u);
		let mut best_diff = 100000000f32;
		for i in range(0,elements.len()-1) {
			let ela = elements.get(i);
			for j in range(i+1,elements.len()) {
				let elb = elements.get(j);
				let diff = ela.active.get_distance(&elb.active);
				if diff < best_diff {
					best_diff = diff;
					best_id = (i,j);
				}
			}
		}
		{
			let (i,j) = best_id;
			let removed = elements.remove(j).unwrap();
			*elements.get_mut(i) = Element::new_merge(elements.get(i), &removed);
		}
		if dump_id<dump_numbers.len() && dump_numbers[dump_id] == elements.len() {
			dump_id += 1;
			elements.sort_by(|a,b| b.m0.cmp(&a.m0));
			println!("Dumping at {}", elements.len());
			let path = std::path::Path::new(format!("dump-{}.txt", elements.len()));
			let mut out = io::BufferedWriter::new(io::File::create(&path));
			for el in elements.iter() {
				let disp = el.m2.div(el.m0).sub( &el.active.sqr() );
				out.write_str(format!(
					"Group of {:u}\n\tDistLog: {}\t({})\n\tSymbol: {}\t({})\n",
					el.m0, el.active.dist_log, disp.dist_log,
					el.active.symbol, disp.symbol
					)).unwrap();
				out.write_str(format!(
					"\tRank avg: {}\t({})\n\tDistLimLog: {}\t({})\n\tDistLog avg: {}\t({})\n",
					el.active.rank_avg, disp.rank_avg,
					el.active.dist_lim_log, disp.dist_lim_log,
					el.active.dist_log_avg, disp.dist_log_avg
					)).unwrap();
			}
		}
	}
	println!("Done.");
}
