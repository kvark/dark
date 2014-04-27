#![crate_id = "cluster"]
#![crate_type = "bin"]

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

fn process_brute(values: Vec<Value>, dump_numbers: ~[uint]) {
	let mut elements: Vec<Element> = values.iter().map(|v| {
		let sqr = v.sqr();
		Element {
			m0	: 1,
			m1	: v.clone(),
			m2	: sqr,
			active	: v.clone(),
		}
	}).collect();
	let mut dump_id = 0u;
	while elements.len()>1 {
		println!("Starting {}", elements.len());
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
}


#[deriving(Clone)]
struct ContextRef {
	symbol: uint,
	rank: uint,
}

static rank_limits: [uint,..9] = [1u,2u,4u,8u,16u,32u,64u,128u,256u];

impl ContextRef {
	fn get_limit() -> uint {
		rank_limits.len() << 8
	}
	fn new(v: &Value) -> ContextRef {
		ContextRef {
			symbol: v.symbol as uint,
			rank: rank_limits.iter().position(|&rl| v.last_rank<rl).unwrap(),
		}
	}
	fn encode(&self) -> uint {
		(self.symbol) + (self.rank<<8)
	}
	fn decode(id: uint) -> ContextRef {
		ContextRef {
			symbol: id & 0xFF,
			rank: id>>8,
		}
	}
}

#[deriving(Clone)]
struct DistSet {
	m0: f32,
	m1: f32,
	m2: f32,
	avg: f32,
}

impl DistSet {
	fn new(v: &Value) -> DistSet {
		let x = (v.dist_log + 1.0).ln();
		DistSet {
			m0: 1.0,
			m1: x,
			m2: x*x,
			avg: x,
		}
	}
	fn add_self(&mut self, other: &DistSet) {
		self.m0 += other.m0;
		self.m1 += other.m1;
		self.m2 += other.m2;
		self.avg = self.m1 / self.m0;
	}
	fn get_variance(&self) -> f32 {
		self.m2 / self.m0 - self.avg*self.avg
	}
	fn get_distance(&self, other: &DistSet) -> f32 {
		let d1 = self.avg - other.avg;
		let d2 = self.get_variance() - other.get_variance();
		d1*d1 + d2*d2
	}
}

struct Group {
	dist	: DistSet,
	cells	: Vec<ContextRef>,
}

impl Group {
	fn consume(&mut self, other: Group) {
		self.dist.add_self(&other.dist);
		self.cells.push_all(other.cells.as_slice());
	}
}

fn process_cell(values: Vec<Value>, dump_numbers: ~[uint]) {
	// populate groups
	let mut dset: Vec<DistSet> = Vec::from_fn(ContextRef::get_limit(), |_|
		DistSet{ m0:0.0, m1:0.0, m2:0.0, avg:0.0 });
	for v in values.iter() {
		let id = ContextRef::new(v).encode();
		dset.get_mut(id).add_self(&DistSet::new(v));
	}
	let mut dump_id = 0u;
	let mut groups: Vec<Group> = dset.iter().enumerate().filter(|&(_,dv)| {
		dv.m0 > 0.0
	}).map(|(i,dv)| {
		Group {
			dist: dv.clone(),
			cells: Vec::from_fn(1, |_| ContextRef::decode(i)),
		}
	}).collect();
	// merge iteratively
	println!("Base {}/{} groups", groups.len(), ContextRef::get_limit());
	while groups.len() > 1 {
		let mut best_id = (0u,0u);
		let mut best_diff = 100000000f32;

		for i in range(0,groups.len()-1) {
			let ga = groups.get(i);
			for j in range(i+1,groups.len()) {
				let gb = groups.get(j);
				let diff = ga.dist.get_distance(&gb.dist);
				if diff < best_diff {
					best_diff = diff;
					best_id = (i,j);
				}
			}
		}
		{
			let (i,j) = best_id;
			let removed = groups.remove(j).unwrap();
			groups.get_mut(i).consume(removed);
		}
		if dump_id<dump_numbers.len() && dump_numbers[dump_id] == groups.len() {
			dump_id += 1;
			groups.sort_by(|a,b| {
				if b.dist.m0 < a.dist.m0 {Less} else if b.dist.m0 > a.dist.m0 {Greater} else {Equal}
			});
			println!("Dumping at {}", groups.len());
			let path = std::path::Path::new(format!("dump-{}.txt", groups.len()));
			let mut out = io::BufferedWriter::new(io::File::create(&path));
			for g in groups.iter() {
				out.write_str(format!(
					"Group of {} ({})\n\tDistLog: {}\t({})\n",
					g.cells.len(), g.dist.m0, g.dist.avg, g.dist.get_variance()
					)).unwrap();
			}
		}
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
		process_brute(values, dump_numbers);
	}else if true {
		process_cell(values, dump_numbers);
	}else {
		process_print(values);
	}
	println!("Done.");
}
