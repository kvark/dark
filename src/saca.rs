/*!

Suffix Array Construction

worst time: O(N)
worst space: N bytes (for input) + N words (for suffix array)

# Credit
Based on the work by Ge Nong team:
https://code.google.com/p/ge-nong/

*/

use std::{iter, vec};

pub type Symbol = u8;
pub type Suffix = u32;

static SUF_INVALID	: Suffix = -1;


fn sort_direct<T: TotalOrd>(input: &[T], suffixes: &mut [Suffix]) {
	for (i,p) in suffixes.mut_iter().enumerate() {
		*p = i as Suffix;
	}

	suffixes.sort_by(|&a,&b| {
		iter::order::cmp(
			input.slice_from(a as uint).iter(),
			input.slice_from(b as uint).iter())
	});

	debug!("sort_direct: {:?}", suffixes);
}


fn fill<T: Pod>(slice: &mut [T], value: T) {
	for elem in slice.mut_iter() {
		*elem = value;
	}
}

fn get_buckets<T: ToPrimitive>(input: &[T], buckets: &mut [Suffix], end: bool) {
	fill(buckets, 0);

	for sym in input.iter() {
		buckets[sym.to_uint().unwrap()] += 1;
	}

	//let mut sum = 1u;	// Sentinel is below
	let mut sum = 0 as Suffix;
	for buck in buckets.mut_iter() {
		sum += *buck;
		*buck = if end {sum} else {sum - *buck};
	}
}

/// Fill LMS strings into the beginning of their buckets
fn put_substr<T: Eq + Ord + ToPrimitive>(suffixes: &mut [Suffix], input: &[T], buckets: &mut [Suffix]) {
	// Find the end of each bucket.
	get_buckets(input, buckets, true);
	
	// Set each item in SA as empty.
	fill(suffixes, SUF_INVALID);
	// Active suffixes have +1 value added to them

	// Last string is L-type
	let succ_t = input.iter().zip(input.slice_from(1).iter()).enumerate().rev().fold(false, |succ_t, (i,(prev,cur))| {
		if *prev < *cur || (*prev == *cur && succ_t) {
			true
		}else {
			if succ_t { // LMS detected
				let buck = &mut buckets[cur.to_uint().unwrap()];
				*buck -= 1;
				suffixes[*buck] = (i+1) as Suffix;
				debug!("\tput_substr: detected LMS suf[{}] of symbol '{}', value {}",
					*buck, cur.to_uint().unwrap(), i+1);
			}
			false
		}
	});

	if succ_t && false { // no use for value 0 for the induce_l0
		let cur = &input[0];
		let buck = &mut buckets[cur.to_uint().unwrap()];
		*buck -= 1;
		suffixes[*buck] = 0;
		debug!("\tput_substr: detected LMS suf[{}] of symbol '{}', value {}",
			*buck, cur.to_uint().unwrap(), 0);
	}
}


/// Induce L-type strings
fn induce_low<T: Ord + ToPrimitive>(suffixes: &mut [Suffix], input: &[T], buckets: &mut [Suffix], clean: bool) {
	// Find the head of each bucket.
	get_buckets(input, buckets, false);

	// Process sentinel as L-type (NEW)
	{
		let sym = input.last().unwrap();
		let buck = &mut buckets[sym.to_uint().unwrap()];
		debug!("\tinduce_low: induced suf[{}] of last symbol '{}' to value {}",
			*buck, sym.to_uint().unwrap(), input.len()-1);
		suffixes[*buck] = (input.len()-1) as Suffix;
		*buck += 1;
	}

	for i in range(0, suffixes.len()) {
		let suf = suffixes[i];
		if suf == SUF_INVALID || suf == 0 {continue}
		let sym = &input[suf-1];
		if *sym >= input[suf] { // L-type
			let buck = &mut buckets[sym.to_uint().unwrap()];
			debug!("\tinduce_low: induced suf[{}] of symbol '{}' to value {}",
				*buck, sym.to_uint().unwrap(), suf-1);
			if !clean || suf != 1 {	//we don't want anything at 0 now
				suffixes[*buck] = suf-1;
			}
			*buck += 1;
			if clean {
				suffixes[i] = SUF_INVALID;
			}
		}
	}

	debug!("induce_low: result suf {:?}", suffixes);
}

/// Induce S-type strings
fn induce_sup<T: Ord + ToPrimitive>(suffixes: &mut [Suffix], input: &[T], buckets: &mut [Suffix], clean: bool) {
	// Find the head of each bucket.
	get_buckets(input, buckets, true);

	for i in range(0, suffixes.len()).rev() {
		let suf = suffixes[i];
		if suf == SUF_INVALID || suf == 0 {continue}
		let sym = &input[suf-1];
		let buck = &mut buckets[sym.to_uint().unwrap()];
		if *sym <= input[suf] && *buck as uint <= i { // S-type
			assert!(*buck>0, "Invalid bucket for symbol {} at suffix {}",
				sym.to_uint().unwrap(), suf);
			*buck -= 1;
			suffixes[*buck] = suf-1;
			debug!("\tinduce_sup: induced suf[{}] of symbol '{}' to value {}",
				*buck, sym.to_uint().unwrap(), suf-1);
			if clean {
				suffixes[i] = SUF_INVALID;
			}
		}
	}

	debug!("induce_sup: result suf {:?}", suffixes);
}

fn get_lms_length<T: Eq + Ord>(input: &[T]) -> uint {
	if input.len() == 1 {
		return 1
	}

	let mut dist = 0u;
	let mut i = 0u;
	while {i+=1; i<input.len() && input[i-1] <= input[i]} {}
	
	loop {
		if i >= input.len()-0 || input[i-1] < input[i] {break}
		if i == input.len()-1 || input[i-1] > input[i] {dist=i}
		i += 1;
	}

	dist+1
}

fn name_substr<T: Eq + Ord>(suffixes: &mut [Suffix], n1: uint, input: &[T]) -> uint {
	// Init the name array buffer.
	fill(suffixes.mut_slice_from(n1), SUF_INVALID);

 	// Scan to compute the interim s1.
	let mut pre_pos = 0u;
	let mut pre_len = 0u;
	let mut name = 0u;
	let mut name_count = 0u;
	for i in range(0, n1) {
		let pos = suffixes[i] as uint;
		let len = get_lms_length(input.slice_from(pos));
		debug!("\tLMS at {} has length {}", pos, len);
		if len != pre_len || input.slice(pre_pos, pre_pos+len) != input.slice(pos, pos+len) {
			name = i;	// A new name.
			name_count += 1;
			suffixes[name] = 1;
			pre_pos = pos;
			pre_len = len;
		}else {
			suffixes[name] += 1;	// Count this name.
		}
		suffixes[n1 + (pos>>1)] = name as Suffix;
	}

	// Compact the interim s1 sparsely stored in SA[n1, n-1] into SA[0, n1].
	let mut j = 0;
	for i in range(n1, suffixes.len()) {
		if suffixes[i] != SUF_INVALID {
			let value = suffixes[j];
			suffixes[j] = suffixes[i];
			suffixes[n1+j] = value;
			j += 1;
		}
	}

	assert_eq!(j, n1);
	debug!("names: {:?}", suffixes.slice_to(n1));
	debug!("names counts: {:?}", suffixes.slice(n1, n1+name_count));
	name_count
}

fn rename_substr(suffixes: &[Suffix], input: &mut [Suffix]) {
	// Rename each S-type character of the interim s1 as the end
	// of its bucket to produce the final s1.
	range(1, input.len()).rev().fold(true, |succ_t, i| {
		let prev = input[i-1];
		let cur = input[i];
		if prev < cur || (prev == cur && succ_t) {
			input[i-1] += suffixes[prev] - 1;
			true
		}else {
			false
		}
	});
}

fn gather_lms<T: Eq + Ord>(sa_new: &mut [Suffix], input_new: &mut [Suffix], input: &[T]) {
	let mut j = input_new.len();
	
	// s[n-2] must be L-type
	let succ_t = input.iter().zip(input.slice_from(1).iter()).enumerate().rev().fold(false, |succ_t, (i,(prev,cur))| {
		if *prev < *cur || (*prev == *cur && succ_t) {
			true
		}else {
			if succ_t {
				j -= 1;
				input_new[j] = (i+1) as Suffix;
				debug!("\tgather_lms: found suffix {} as input[{}]", i+1, j);
			}
			false
		}
	});

	if succ_t {
		j -= 1;
		input_new[j] = 0;
		debug!("\tgather_lms: found first suffix as input[{}]", j);
	}
	assert!(j == 0);
	
	for suf in sa_new.mut_iter() {
		*suf = input_new[*suf];
	}
}

fn put_suffix<T: ToPrimitive>(suffixes: &mut [Suffix], n1: uint, input: &[T], buckets: &mut [Suffix]) {
	// Find the end of each bucket.
	get_buckets(input, buckets, true);

	//TEMP: copy suffixes to the beginning of the list
	for i in range(0,n1) {
		suffixes[i] = suffixes[n1+i];
		suffixes[n1+i] = SUF_INVALID;
	}

	for i in range(0,n1).rev() {
		let p = suffixes[i];
		assert!(p != SUF_INVALID);
		suffixes[i] = SUF_INVALID;
		let sym = &input[p];
		let buck = &mut buckets[sym.to_uint().unwrap()];
		*buck -= 1;
		assert!(*buck as uint >= i);
		suffixes[*buck] = p;
	}

	debug!("put_suffix: {:?}", suffixes);
}


fn saca<T: Eq + Ord + ToPrimitive>(input: &[T], suf_and_buckets: &mut [Suffix]) {
	debug!("saca: entry");
	debug!("input len: {}, excess: {}", input.len(), suf_and_buckets.len() - input.len());
	assert!(input.len() <= suf_and_buckets.len());

	// Stage 1: reduce the problem by at least 1/2.
	let n1 = {
		let (suffixes, buckets) = suf_and_buckets.mut_split_at(input.len());
		put_substr(suffixes, input, buckets);
		induce_low(suffixes, input, buckets, true);
		induce_sup(suffixes, input, buckets, true);

		// Now, all the LMS-substrings are sorted and stored sparsely in SA.
		// Compact all the sorted substrings into the first n1 items of SA.
		let mut n1 = 0u;
		for i in range(0, suffixes.len()) {
			if suffixes[i] != SUF_INVALID {
				suffixes[n1] = suffixes[i];
				n1 += 1;
			}
		}
		debug!("Compacted LMS: {:?}", suffixes.slice_to(n1));
		n1
	};
	
	// Stage 2: solve the reduced problem.
	{
		assert!(n1+n1 <= input.len());
		let num_names = name_substr(suf_and_buckets, n1, input);
		debug!("num_names = {}", num_names);
		assert!(n1+n1+num_names <= suf_and_buckets.len())
		let (input_new, sa_new) = suf_and_buckets.mut_split_at(n1);
		rename_substr(sa_new, input_new);
		debug!("renamed sa_new: {:?}", sa_new.slice_to(num_names));
		debug!("renamed input_new: {:?}", input_new);
		fill(sa_new, SUF_INVALID);

		if num_names < n1 {
			// Recurse if names are not yet unique.
			saca(input_new, sa_new);
		}else {
			// Get the suffix array of s1 directly.
			for (i,&sym) in input_new.iter().enumerate() {
				sa_new[sym] = i as Suffix;
			}
			debug!("Sorted suffixes: {:?}", sa_new.slice_to(n1));
		}

		let slice = sa_new.mut_slice_to(n1);
		gather_lms(slice, input_new, input);
		fill(input_new, SUF_INVALID);
		debug!("Gathered LMS: {:?}", slice);
	}

	// Stage 3: induce SA(S) from SA(S1).
	{
		let (suffixes, buckets) = suf_and_buckets.mut_split_at(input.len());
		put_suffix(suffixes, n1, input, buckets);
		induce_low(suffixes, input, buckets, false);
		induce_sup(suffixes, input, buckets, false);
	}
}


/// Suffix Array Constructor
pub struct Constructor {
	priv suffixes	: ~[Suffix],
	priv n			: uint,
}

impl Constructor {
	/// Create a new instance for a given maximum input size
	pub fn new(max_n: uint) -> Constructor {
		let add = 1u<<15;
		Constructor {
			suffixes: vec::from_elem(max_n+add, 0 as Suffix),
			n		: max_n,
		}
	}

	/// Compute the suffix array for a given input
	pub fn compute<'a>(&'a mut self, input: &[Symbol]) -> &'a [Suffix] {
		assert_eq!(input.len(), self.n);
		if true {
			saca(input, self.suffixes);
		}else {
			sort_direct(input, self.suffixes);
		}

		debug!("construct suf: {:?}", self.suffixes.slice_to(self.n));
		self.suffixes.slice_to(self.n)
	}

	/// Temporarily provide the storage for outside needs
	pub fn reuse<'a>(&'a mut self) -> &'a mut [Suffix] {
		self.suffixes.as_mut_slice()
	}
}


#[cfg(test)]
pub mod test {
	use compress::bwt;

	fn some_detail(input: &[super::Symbol], suf_expected: &[super::Suffix], origin_expected: uint, out_expected: &[super::Symbol]) {
		let mut con = super::Constructor::new(input.len());
		let (output, origin) = {
			let suf = con.compute(input);
			assert_eq!(suf.as_slice(), suf_expected);
			let mut iter = bwt::TransformIterator::new(input, suf);
			let out = iter.by_ref().to_owned_vec();
			(out, iter.get_origin())
		};
		assert_eq!(origin, origin_expected);
		assert_eq!(output.as_slice(), out_expected);
		let suf = con.reuse().mut_slice_to(input.len());
		let decoded = bwt::decode(output, origin, suf).to_owned_vec();
		assert_eq!(input.as_slice(), decoded.as_slice());
	}

	#[test]
	fn detailed() {
		some_detail(bytes!("abracadabra"), [10,7,0,3,5,8,1,4,6,9,2], 2, bytes!("rdarcaaaabb"));
		some_detail(bytes!("banana"), [5,3,1,0,4,2], 3, bytes!("nnbaaa"));
	}

	fn some_roundtrip(input: &[super::Symbol]) {
		debug!("Roundtrip {:?}", input);
		let mut con = super::Constructor::new(input.len());
		let (output, origin) = {
			let suf = con.compute(input);
			let mut iter = bwt::TransformIterator::new(input, suf);
			let out = iter.by_ref().to_owned_vec();
			(out, iter.get_origin())	
		};
		let decoded = bwt::decode(output, origin, con.reuse().mut_slice_to(input.len())).
			take(input.len()).to_owned_vec();
		assert_eq!(input.as_slice(), decoded.as_slice());
	}

	#[test]
	fn roundtrips() {
		some_roundtrip([0,1,5,4,3,0]);
		//some_roundtrip(include_bin!("../LICENSE"))
	}
}
