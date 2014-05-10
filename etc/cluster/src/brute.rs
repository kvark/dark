//! Brute-force cluster analysis

use std;
use std::vec::Vec;
use super::Value;


impl super::Value {
	fn get_distance(&self, other: &Value) -> f32 {
		(self.dist_log - other.dist_log).abs() +
		(if self.symbol == other.symbol {0f32} else {1f32}) +
		(if self.rank_avg == other.rank_avg {0f32} else {0.4f32}) +
		(self.dist_lim_log - other.dist_lim_log).abs() * 0.2f32 +
		(self.dist_log_avg - other.dist_log_avg).abs()
	}

	fn add(&self, other: &Value) -> Value {
		super::Value {
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
		super::Value {
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
		super::Value {
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
		super::Value {
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


pub fn process(values: Vec<Value>, args: &[~str]) {
	let dump_numbers: ~[uint] = args.iter().map(|s| from_str(*s).unwrap()).collect();
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
			let mut out = std::io::BufferedWriter::new(std::io::File::create(&path));
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

