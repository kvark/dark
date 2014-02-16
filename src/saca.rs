/*!

Suffix Array Construction

worst time: O(N)
worst space: N bytes (for input) + N words (for suffix array)

# Credit
Ge Nong and the team:
https://code.google.com/p/ge-nong/

*/

use std::{iter, vec};
use compress::bwt;

pub type Symbol = u8;
pub type Suffix = u32;

static SUF_INVALID	: Suffix = 0xFFFFFFFF;


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

fn get_buckets(input: &[Symbol], buckets: &mut [uint], end: bool) {
	fill(buckets, 0);

	for sym in input.iter() {
		buckets[*sym] += 1;
	}

	//let mut sum = 1u;	// Sentinel is below
	let mut sum = 0u;
	for buck in buckets.mut_iter() {
		sum += *buck;
		*buck = if end {sum} else {sum - *buck};
	}
}

/// Fill LMS strings into the beginning of their buckets
fn put_substr0(suffixes: &mut [Suffix], input: &[Symbol], buckets: &mut [uint]) {
	// Find the end of each bucket.
	get_buckets(input, buckets, true);
	
	// Set each item in SA as empty.
	fill(suffixes, SUF_INVALID);
	// Active suffixes have +1 value added to them

	// Last string is L-type
	let succ_t = input.iter().zip(input.slice_from(1).iter()).enumerate().rev().fold(false, |succ_t, (i,(&prev,&cur))| {
		if prev < cur || (prev==cur && succ_t) {
			true
		}else {
			if succ_t { // LMS detected
				buckets[cur] -= 1;
				suffixes[buckets[cur]] = (i+1) as Suffix;
				debug!("\tput_substr0: detected LMS suf[{}] of symbol '{}', value {}",
					buckets[cur], cur as char, i+1);
			}
			false
		}
	});

	//NEW! parse first string as LMS, if it starts with S-type
	if succ_t && false { // no use for value 0 for the induce_l0
		let cur = input[0];
		buckets[cur] -= 1;
		suffixes[buckets[cur]] = 0;
		debug!("\tput_substr0: detected LMS suf[{}] of symbol '{}', value {}",
			buckets[cur], cur as char, 0);
	}

	//TODO: evaluate
	// Set the single sentinel LMS-substring.
	//suffixes[0] = input.len() as Suffix;
}

/// Induce L-type strings
fn induce_l0(suffixes: &mut [Suffix], input: &[Symbol], buckets: &mut [uint], clean: bool) {
	// Find the head of each bucket.
	get_buckets(input, buckets, false);

	// Process sentinel as L-type (NEW)
	{
		let sym = *input.last().unwrap();
		debug!("\tinduce_l0: induced suf[{}] of last symbol '{}' to value {}",
			buckets[sym], sym as char, input.len()-1);
		suffixes[buckets[sym]] = (input.len()-1) as Suffix;
		buckets[sym] += 1;
	}

	//TODO: remove
	//buckets[0] += 1; // skip the virtual sentinel.

	for i in range(0, suffixes.len()) {
		let suf = suffixes[i];
		if suf == SUF_INVALID || suf == 0 {continue}
		let sym = input[suf-1];
		if sym >= input[suf] { // L-type
			debug!("\tinduce_l0: induced suf[{}] of symbol '{}' to value {}",
				buckets[sym], sym as char, suf-1);
			if !clean || suf != 1 {	//we don't want anything at 0 now
				suffixes[buckets[sym]] = suf-1;
			}
			buckets[sym] += 1;
			if clean {
				suffixes[i] = SUF_INVALID;
			}
		}
	}

	debug!("induce_l0: result suf {:?}", suffixes);
}

/// Induce S-type strings
fn induce_s0(suffixes: &mut [Suffix], input: &[Symbol], buckets: &mut [uint], clean: bool) {
	// Find the head of each bucket.
	get_buckets(input, buckets, true);

	for i in range(0,suffixes.len()).rev() {
		let suf = suffixes[i];
		if suf == SUF_INVALID || suf == 0 {continue}
		let sym = input[suf-1];
		if sym <= input[suf] && buckets[sym] <= i { // S-type
			buckets[sym] -= 1;
			suffixes[buckets[sym]] = suf-1;
			debug!("\tinduce_s0: induced suf[{}] of symbol '{}' to value {}",
				buckets[sym], sym as char, suf-1);
			if clean {
				suffixes[i] = SUF_INVALID;
			}
		}
	}

	debug!("induce_s0: result suf {:?}", suffixes);
}

fn get_lms_length<T: Eq + Ord>(input: &[T]) -> uint {
	if input.len() == 1 {
		return 1
	}

	let mut dist = 0u;
	let mut i = 0u;
	while {i+=1; input[i-1] <= input[i]} {}
	
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

	// Compact the interim s1 sparsely stored in SA[n1, n-1] into SA[m-n1, m-1].
	let mut j = suffixes.len();
	for i in range(n1, suffixes.len()).rev() {
		if suffixes[i] != SUF_INVALID {
			j -= 1;
			suffixes[j] = suffixes[i];
		}
	}

	debug!("names counts: {:?}", suffixes.slice_to(name_count));
	debug!("names: {:?}", suffixes.slice_from(j));
	assert!(j+n1 == suffixes.len());

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

fn gather_lms<T: Eq + Ord + Pod>(sa_new: &mut [Suffix], input_new: &mut [Suffix], input: &[T]) {
	let mut j = input_new.len();
	//j -= 1; //TODO: remove
	//input_new[j] = (input.len()-1) as Suffix;
	
	// s[n-2] must be L-type
	let succ_t = input.iter().zip(input.slice_from(1).iter()).enumerate().rev().fold(false, |succ_t, (i,(&prev,&cur))| {
		if prev < cur || (prev == cur && succ_t) {
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

	if succ_t {	//NEW
		j -= 1;
		input_new[j] = 0;
		debug!("\tgather_lms: found fist suffix as input[{}]", j);
	}
	assert!(j == 0);
	
	for suf in sa_new.mut_iter() {
		*suf = input_new[*suf];
	}
}

fn put_suffix0(suffixes: &mut [Suffix], n1: uint, input: &[Symbol], buckets: &mut [uint]) {
	// Find the end of each bucket.
	get_buckets(input, buckets, true);

	for i in range(0,n1).rev() {
		let p = suffixes[i];
		assert!(p != SUF_INVALID);
		suffixes[i] = SUF_INVALID;
		let sym = input[p];
		buckets[sym] -= 1;
		assert!(buckets[sym] >= i);
		suffixes[buckets[sym]] = p;
	}

	debug!("put_suffix0: {:?}", suffixes);

	//TODO: remove
	// Set the single sentinel suffix.
	//suffixes[0] = (input.len()-1) as Suffix;
}


fn saca_k1(input: &[Suffix], suffixes: &mut [Suffix], _buckets: &mut [uint], level: uint) {
	debug!("saca_k: entry level {}", level);

	sort_direct(input, suffixes);

	/*put_substr0(suffixes, input, buckets);
	induce_l0(suffixes, input, buckets, true);
	induce_s0(suffixes, input, buckets, true);

	//TODO
	let n1 = 0u;

	assert!(n1+n1 <= input.len());
	let (sa_new, input_new) = suffixes.mut_split_at(n1);
	//TODO

	// Stage 3: induce SA(S) from SA(S1).
	gather_lms(sa_new, input_new, input);

	fill(input_new, EMPTY);

	unimplemented!()
	//TODO*/
}

fn saca_k0(input: &[Symbol], suffixes: &mut [Suffix], buckets: &mut [uint]) {
	debug!("saca_k0: entry");
	assert!(input.len() <= suffixes.len());
	
	// Stage 1: reduce the problem by at least 1/2.
	put_substr0(suffixes, input, buckets);
	induce_l0(suffixes, input, buckets, true);
	induce_s0(suffixes, input, buckets, true);

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
	
	// Stage 2: solve the reduced problem.
	{
		assert!(n1+n1 <= input.len());
		let num_names = name_substr(suffixes, n1, input);
		let num_suffixes = suffixes.len() - n1;
		debug!("num_names = {}, num_suffixes = {}", num_names, num_suffixes);
		let (sa_temp, input_new) = suffixes.mut_split_at(num_suffixes);
		let (sa_new, unused) = sa_temp.mut_split_at(n1);
		fill(unused, SUF_INVALID);
		rename_substr(sa_new, input_new);
		debug!("renamed sa_new: {:?}", sa_new);
		debug!("renamed input_new: {:?}", input_new);

		if num_names < n1 {
			// Recurse if names are not yet unique.
			saca_k1(input_new, sa_new, buckets, 1);
		}else {
			// Get the suffix array of s1 directly.
			for (i,&sym) in input_new.iter().enumerate() {
				sa_new[sym] = i as Suffix;
			}
		}

		gather_lms(sa_new, input_new, input);
		fill(input_new, SUF_INVALID);
		debug!("Gathered LMS: {:?}", sa_new);
	}

	// Stage 3: induce SA(S) from SA(S1).
	put_suffix0(suffixes, n1, input, buckets);
	induce_l0(suffixes, input, buckets, false);
	induce_s0(suffixes, input, buckets, false);
}


/// main entry point for SAC
pub fn construct_suffix_array(input: &[Symbol], suffixes: &mut [Suffix]) {
	assert_eq!(input.len(), suffixes.len());

	if true {
		let mut buckets = [0u, ..0x100];
		saca_k0(input, suffixes, buckets.as_mut_slice());
	}else {
		sort_direct(input, suffixes);
	}

	debug!("construct suf: {:?}", suffixes);
}


/// An iterator over BWT output
pub struct LastColumnIterator<'a> {
	priv input		: &'a [Symbol],
	priv suf_iter	: iter::Enumerate<vec::Items<'a,Suffix>>,
	priv origin		: Option<uint>,
}

impl<'a> LastColumnIterator<'a> {
	/// create a new BWT iterator from the suffix array
	pub fn new(input: &'a [Symbol], suffixes: &'a [Suffix]) -> LastColumnIterator<'a> {
		LastColumnIterator {
			input: input,
			suf_iter: suffixes.iter().enumerate(),
			origin: None,
		}
	}

	/// return the index of the original string
	pub fn get_origin(&self) -> uint {
		self.origin.unwrap()
	}
}

impl<'a> Iterator<Symbol> for LastColumnIterator<'a> {
	fn next(&mut self) -> Option<Symbol> {
		self.suf_iter.next().map(|(i,&p)| {
			if p == 0 {
				assert!( self.origin.is_none() );
				self.origin = Some(i);
				*self.input.last().unwrap()
			}else {
				self.input[p-1]
			}
		})
	}
}


/// A helper method to perform BWT on a given block and place the result into output
/// returns the original string index in the sorted matrix
pub fn BW_transform(input: &[Symbol], suf: &mut [Suffix], output: &mut [Symbol]) -> uint {
	construct_suffix_array(input, suf);
	let mut iter = LastColumnIterator::new(input, suf);
	for (out,s) in output.mut_iter().zip(iter.by_ref()) {
		*out = s;
	}
	iter.get_origin()
}


/// An iterator over inverse BWT
pub struct InverseIterator<'a> {
	priv input		: &'a [Symbol],
	priv suffixes	: &'a [Suffix],
	priv origin		: uint,
	priv current	: Suffix,
}

impl<'a> InverseIterator<'a> {
	/// create a new inverse BWT iterator
	pub fn new(input: &'a [Symbol], origin: uint, suffixes: &'a mut [Suffix]) -> InverseIterator<'a> {
		assert_eq!(input.len(), suffixes.len())
		debug!("decode origin={}, input: {:?}", origin, input)

		let mut radix = bwt::Radix::new();
		radix.gather(input);
		radix.accumulate();

		suffixes[radix.place(input[origin])] = 0;
		for (i,&ch) in input.slice_to(origin).iter().enumerate() {
			suffixes[radix.place(ch)] = (i+1) as Suffix;
		}
		for (i,&ch) in input.slice_from(origin+1).iter().enumerate() {
			suffixes[radix.place(ch)] = (origin+2+i) as Suffix;
		}
		//suffixes[-1] = origin;
		debug!("decode table: {:?}", suffixes)

		InverseIterator {
			input: input,
			suffixes: suffixes,
			origin: origin,
			current: origin as Suffix,
		}
	}
}

impl<'a> Iterator<Symbol> for InverseIterator<'a> {
	fn next(&mut self) -> Option<Symbol> {
		if self.current == -1 {
			None
		}else {
			self.current = self.suffixes[self.current] - 1;
			debug!("\tjumped to {}", self.current);
			let p = if self.current!=-1 {
				self.current
			}else {
				self.origin as Suffix
			};
			Some(self.input[p])
		}
	}
}

#[cfg(test)]
pub mod test {
	use std::vec;
	use super::{SUF_INVALID, Suffix, Symbol};

	#[test]
	fn consts() {
		assert_eq!(SUF_INVALID, -1 as Suffix);
	}

	fn some_detail(input: &[Symbol], suf_expected: &[Suffix], origin_expected: uint, out_expected: &[Symbol]) {
		let mut suf = vec::from_elem(input.len(), 0 as Suffix);
		super::construct_suffix_array(input, suf);
		assert_eq!(suf.as_slice(), suf_expected);
		let (output, origin) = {
			let mut iter = super::LastColumnIterator::new(input,suf);
			let out = iter.by_ref().take(input.len()).to_owned_vec();
			(out, iter.get_origin())
		};
		assert_eq!(origin, origin_expected);
		assert_eq!(output.as_slice(), out_expected);
		let decoded = super::InverseIterator::new(output, origin, suf).to_owned_vec();
		assert_eq!(input.as_slice(), decoded.as_slice());
	}

	#[test]
	fn detailed() {
		some_detail(bytes!("abracadabra"), [10,7,0,3,5,8,1,4,6,9,2], 2, bytes!("rdarcaaaabb"));
		some_detail(bytes!("banana"), [5,3,1,0,4,2], 3, bytes!("nnbaaa"));
	}

	/*#[test]
	fn roundtrip() {
		let input = include_bin!("../LICENSE");
		let mut suf = vec::from_elem(input.len(), 0 as Suffix);
		let mut output = vec::from_elem(input.len(), 0 as Symbol);
		let origin = super::BW_transform(input, suf, output);
		let decoded = super::InverseIterator::new(output, origin, suf).
			take(input.len()).to_owned_vec();
		assert_eq!(input.as_slice(), decoded.as_slice());
	}*/
}
