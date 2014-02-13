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


/// main entry point for SAC
pub fn construct_suffix_array(input: &[Symbol], suffixes: &mut [Suffix]) {
	assert_eq!(input.len(), suffixes.len());
	for (i,p) in suffixes.mut_iter().enumerate() {
		*p = i as Suffix;
	}

	suffixes.sort_by(|&a,&b| {
		iter::order::cmp(
			input.slice_from(a).iter(),
			input.slice_from(b).iter())
	});

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
	use std::vec;
	use super::{Suffix, Symbol};

	#[test]
	fn some() {
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
