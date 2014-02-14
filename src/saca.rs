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
pub type Suffix = uint;

static EMPTY: Suffix = 0x7FFFFFFF;


fn get_buckets(input: &[Symbol], buckets: &mut [uint], end: bool) {
	for buck in buckets.mut_iter() {
		*buck = 0;
	}
	for sym in input.iter() {
		buckets[*sym] += 1;
	}
	let mut sum = 0u;
	for buck in buckets.mut_iter() {
		sum += *buck;
		*buck = if end {sum} else {sum - *buck};
	}
}

fn put_substr0(_suffixes: &mut [Suffix], input: &[Symbol], buckets: &mut [uint]) {
	// Find the end of each bucket.
	get_buckets(input, buckets, true);

	//TODO
}

fn induce_l0(_suffixes: &mut [Suffix], _input: &[Symbol], _buckets: &mut [uint], _clean: bool) {
	//TODO
}

fn induce_s0(_suffixes: &mut [Suffix], _input: &[Symbol], _buckets: &mut [uint], _clean: bool) {
	//TODO
}

fn gather_lms(suffixes: &mut [Suffix], input: &[Suffix]) {
	//TODO
	
	for suf in suffixes.mut_iter() {
		*suf = input[*suf];
	}
}

fn put_suffix0(suffixes: &mut [Suffix], n1: uint, input: &[Symbol], buckets: &mut [uint]) {
	// Find the end of each bucket.
	get_buckets(input, buckets, true);

	for i in range(0,n1).rev() {
		let p = suffixes[i];
		suffixes[i] = 0;
		let sym = input[p];
		buckets[sym] -= 1;
		suffixes[buckets[sym]] = p;
	}

	// Set the single sentinel suffix.
	suffixes[0] = input.len()-1;
}

fn name_substr(_suffixes: &mut [Suffix], _input: &[Symbol], _new_input: &[Suffix], _level: uint) -> uint {
	0 //TODO
}

fn saca_kn(input: &[Suffix], suffixes: &mut [Suffix], _level: uint) {
	//TODO

	let n1 = 0u;

	assert!(n1+n1 <= input.len());
	let (sa_new, input_new) = suffixes.mut_split_at(n1);
	//TODO

	// Stage 3: induce SA(S) from SA(S1).
	gather_lms(sa_new, input_new);
	for s in input_new.mut_iter() {
		*s = EMPTY;
	}

	//TODO
}

fn saca_k0(input: &[Symbol], K: uint, suffixes: &mut [Suffix]) {
	let mut buckets = vec::from_elem(K, 0u);

	// Stage 1: reduce the problem by at least 1/2.
	put_substr0(suffixes, input, buckets.as_mut_slice());
	induce_l0(suffixes, input, buckets.as_mut_slice(), false);
	induce_s0(suffixes, input, buckets.as_mut_slice(), false);

	// Now, all the LMS-substrings are sorted and stored sparsely in SA.
	// Compact all the sorted substrings into the first n1 items of SA.
	let mut n1 = 0u;
	for i in range(0, suffixes.len()) {
		if suffixes[i]>0 {
			suffixes[n1] = suffixes[i];
			n1 += 1;
		}
	}
	
	// Stage 2: solve the reduced problem.
	{
		assert!(n1+n1 <= input.len());
		let (sa_new, input_new) = suffixes.mut_split_at(n1);
		let num_names = name_substr(sa_new, input, input_new, 0);

		if num_names < n1 {
			// Recurse if names are not yet unique.
			saca_kn(input_new, sa_new, 1);
		}else {
			// Get the suffix array of s1 directly.
			for (i,&sym) in input_new.iter().enumerate() {
				sa_new[sym] = i;
			}
		}

		// Stage 3: induce SA(S) from SA(S1).
		gather_lms(sa_new, input_new);
		for s in input_new.mut_iter() {
			*s = 0;
		}
	}

	put_suffix0(suffixes, n1, input, buckets.as_mut_slice());
	induce_l0(suffixes, input, buckets.as_mut_slice(), true);
	induce_s0(suffixes, input, buckets.as_mut_slice(), true);
}


/// main entry point for SAC
pub fn construct_suffix_array(input: &[Symbol], suffixes: &mut [Suffix]) {
	assert_eq!(input.len(), suffixes.len());
	for (i,p) in suffixes.mut_iter().enumerate() {
		*p = i as Suffix;
	}

	if true {
		saca_k0(input, 0x100, suffixes);
	}else {
		suffixes.sort_by(|&a,&b| {
			iter::order::cmp(
				input.slice_from(a).iter(),
				input.slice_from(b).iter())
		});
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
	priv origin		: Suffix,
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
			suffixes[radix.place(ch)] = i+1;
		}
		for (i,&ch) in input.slice_from(origin+1).iter().enumerate() {
			suffixes[radix.place(ch)] = origin+2+i;
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
			let p = if self.current!=-1 {self.current} else {self.origin};
			Some(self.input[p])
		}
	}
}

#[cfg(test)]
pub mod test {
	use std::{mem, vec};
	use super::{EMPTY, Suffix, Symbol};

	#[test]
	fn consts() {
		assert_eq!(EMPTY, 1<<(mem::size_of::<Suffix>()*8-1));
	}

	#[test]
	fn abracadabra() {
		let input = bytes!("abracadabra");
		let mut suf = vec::from_elem(input.len(), 0 as Suffix);
		super::construct_suffix_array(input, suf);
		assert_eq!(suf.as_slice(), &[10,7,0,3,5,8,1,4,6,9,2]);
		let (output, origin) = {
			let mut iter = super::LastColumnIterator::new(input,suf);
			let out = iter.by_ref().take(input.len()).to_owned_vec();
			(out, iter.get_origin())
		};
		assert_eq!(origin, 2);
		let expected = bytes!("rdarcaaaabb");
		assert_eq!(output.as_slice(), expected.as_slice());
		let decoded = super::InverseIterator::new(output, origin, suf).to_owned_vec();
		assert_eq!(input.as_slice(), decoded.as_slice());
	}

	#[test]
	fn roundtrip() {
		let input = include_bin!("../LICENSE");
		let mut suf = vec::from_elem(input.len(), 0 as Suffix);
		let mut output = vec::from_elem(input.len(), 0 as Symbol);
		let origin = super::BW_transform(input, suf, output);
		let decoded = super::InverseIterator::new(output, origin, suf).
			take(input.len()).to_owned_vec();
		assert_eq!(input.as_slice(), decoded.as_slice());
	}
}
