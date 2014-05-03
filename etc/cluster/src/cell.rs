//! Cell-based cluster analysis

use std;
use std::num::Float;
use std::vec::Vec;
use super::Value;


#[deriving(Clone)]
struct ContextRef {
	symbol	: uint,
	rank	: uint,
	avg_dist: uint,
}

static rank_limits: [uint,..10] = [0, 1, 2, 4, 8, 16, 32, 64, 128, 25];
static dist_limits: [f32,..9] = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];

impl ContextRef {
	fn get_limit() -> uint {
		dist_limits.len() * rank_limits.len() << 8
	}
	fn new(v: &Value) -> ContextRef {
		let dlog = (v.dist_log+1.0).ln();
		ContextRef {
			symbol	: v.symbol as uint,
			rank	: rank_limits.iter().position(|&rl| v.last_rank<rl).unwrap(),
			avg_dist: dist_limits.iter().position(|&dl| dlog<dl).unwrap(),
		}
	}
	fn encode(&self) -> uint {
		(self.symbol) + (self.rank<<8) + (self.avg_dist<<8)*rank_limits.len()
	}
	fn decode(id: uint) -> ContextRef {
		ContextRef {
			symbol	: id & 0xFF,
			rank	: (id>>8)%rank_limits.len(),
			avg_dist: (id>>8)/rank_limits.len()
		}
	}
}

impl std::fmt::Show for ContextRef {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f.buf, "Context(sym:{}, rank:{}-{}, dist:{}-{})",
			self.symbol, rank_limits[self.rank], rank_limits[self.rank+1],
			dist_limits[self.avg_dist].exp(),
			dist_limits[self.avg_dist+1].exp())
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


pub fn process(values: Vec<Value>, dump_numbers: ~[uint]) {
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
			let mut out = std::io::BufferedWriter::new(std::io::File::create(&path));
			for g in groups.iter() {
				out.write_str(format!(
					"Group of {} ({})\n\tDistLog: {}\t({})\n",
					g.cells.len(), g.dist.m0, g.dist.avg, g.dist.get_variance()
					)).unwrap();
				for cl in g.cells.iter() {
					out.write_str(format!("\t{}\n", *cl)).unwrap();
				}
			}
		}
	}
}
