/*!

Suffix Array Construction

worst time: O(N)
worst space: N bytes (for input) + N words (for suffix array) + N/4 words (extra)

# Credit
Based on the work by Ge Nong team:
https://code.google.com/p/ge-nong/

*/

use log::LogLevel;
use num::ToPrimitive;

/// Symbol type
pub type Symbol = u8;
/// Suffix type = index of the original sub-string
pub type Suffix = u32;

const SUF_INVALID: Suffix = !0;


fn sort_direct<T: Ord>(input: &[T], suffixes: &mut [Suffix]) {
    for (i,p) in suffixes.iter_mut().enumerate() {
        *p = i as Suffix;
    }

    suffixes.sort_by(|&a,&b| {
        input[a as usize..].cmp(&input[b as usize..])
    });

    debug!("sort_direct: {:?}", suffixes);
}


fn fill<T: Copy>(slice: &mut [T], value: T) {
    for elem in slice.iter_mut() {
        *elem = value;
    }
}

fn get_buckets<T: ToPrimitive>(input: &[T], buckets: &mut [Suffix], end: bool) {
    fill(buckets, 0);

    for sym in input.iter() {
        buckets[sym.to_usize().unwrap()] += 1;
    }

    //let mut sum = 1u; // Sentinel is below
    let mut sum = 0 as Suffix;
    for buck in buckets.iter_mut() {
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

    // Last string is L-type
    let succ_t = input.iter()
                      .zip(input[1..].iter())
                      .enumerate().rev()
                      .fold(false, |succ_t, (i,(prev,cur))|
    {
        if *prev < *cur || (*prev == *cur && succ_t) {
            true
        }else {
            if succ_t { // LMS detected
                let buck = &mut buckets[cur.to_usize().unwrap()];
                *buck -= 1;
                suffixes[*buck as usize] = (i+1) as Suffix;
                debug!("\tput_substr: detected LMS suf[{}] of symbol '{}', value {}",
                    *buck, cur.to_usize().unwrap(), i+1);
            }
            false
        }
    });

    if succ_t && false { // no use for value 0 for the induce_l0
        let cur = &input[0];
        let buck = &mut buckets[cur.to_usize().unwrap()];
        *buck -= 1;
        suffixes[*buck as usize] = 0;
        debug!("\tput_substr: detected LMS suf[{}] of symbol '{}', value {}",
            *buck, cur.to_usize().unwrap(), 0);
    }
}


/// Induce L-type strings
fn induce_low<T: Ord + ToPrimitive>(suffixes: &mut [Suffix], input: &[T],
              buckets: &mut [Suffix], clean: bool)
{
    // Find the head of each bucket.
    get_buckets(input, buckets, false);

    // Process sentinel as L-type
    {
        let sym = input.last().unwrap();
        let buck = &mut buckets[sym.to_usize().unwrap()];
        debug!("\tinduce_low: induced suf[{}] of last symbol '{}' to value {}",
            *buck, sym.to_usize().unwrap(), input.len()-1);
        suffixes[*buck as usize] = (input.len()-1) as Suffix;
        *buck += 1;
    }

    for i in 0 .. suffixes.len() {
        let suf = suffixes[i];
        if suf == SUF_INVALID || suf == 0 {continue}
        let sym = &input[suf as usize - 1];
        if *sym >= input[suf as usize] { // L-type
            let buck = &mut buckets[sym.to_usize().unwrap()];
            debug!("\tinduce_low: induced suf[{}] of symbol '{}' to value {}",
                *buck, sym.to_usize().unwrap(), suf-1);
            if !clean || suf != 1 { //we don't want anything at 0 now
                suffixes[*buck as usize] = suf-1;
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
fn induce_sup<T: Ord + ToPrimitive>(suffixes: &mut [Suffix], input: &[T],
    buckets: &mut [Suffix], clean: bool)
{
    // Find the head of each bucket.
    get_buckets(input, buckets, true);

    for i in (0 .. suffixes.len()).rev() {
        let suf = suffixes[i];
        if suf == SUF_INVALID || suf == 0 {continue}
        let sym = &input[suf as usize - 1];
        let buck = &mut buckets[sym.to_usize().unwrap()];
        if *buck as usize <= i { // S-type
            assert!(*sym <= input[suf as usize]);
            assert!(*buck>0, "Invalid bucket for symbol {} at suffix {}",
                sym.to_usize().unwrap(), suf);
            *buck -= 1;
            suffixes[*buck as usize] = suf-1;
            debug!("\tinduce_sup: induced suf[{}] of symbol '{}' to value {}",
                *buck, sym.to_usize().unwrap(), suf-1);
            if clean {
                suffixes[i] = SUF_INVALID;
            }
        }
    }

    debug!("induce_sup: result suf {:?}", suffixes);
}

fn get_lms_length<T: Eq + Ord>(input: &[T]) -> usize {
    if input.len() == 1 {
        return 1
    }

    let mut dist = 0usize;
    let mut i = 0usize;
    while {i+=1; i<input.len() && input[i-1] <= input[i]} {}

    loop {
        if i >= input.len()-0 || input[i-1] < input[i] {break}
        if i == input.len()-1 || input[i-1] > input[i] {dist=i}
        i += 1;
    }

    dist + 1
}

fn name_substr<T: Eq + Ord>(sa_new: &mut [Suffix], input_new: &mut [Suffix], input: &[T]) -> usize {
    // Init the name array buffer.
    fill(sa_new, SUF_INVALID);

    // Scan to compute the interim s1.
    let mut pre_pos = 0usize;
    let mut pre_len = 0usize;
    let mut name = -1;
    for suf in input_new.iter() {
        let pos = *suf as usize;
        let len = get_lms_length(&input[pos..]);
        debug!("\tLMS at {} has length {}", pos, len);
        if len != pre_len || input[pre_pos .. pre_pos+len] != input[pos .. pos+len] {
            name += 1;  // A new name.
            pre_pos = pos;
            pre_len = len;
        }
        sa_new[pos>>1] = name as Suffix;
    }

    let mut iter = sa_new.iter();
    for value in input_new.iter_mut() {
        *value = *iter.find(|&v| *v != SUF_INVALID).unwrap();
    }

    (name + 1) as usize
}

fn gather_lms<T: Eq + Ord>(input_new: &mut [Suffix], input: &[T]) {
    let mut iter = input_new.iter_mut().rev();

    let succ_t = input.iter()
                      .zip(input[1..].iter())
                      .enumerate().rev()
                      .fold(false, |succ_t, (i,(prev,cur))|
    {
        if *prev < *cur || (*prev == *cur && succ_t) {
            true
        }else {
            if succ_t {
                *iter.next().unwrap() = (i+1) as Suffix;
                debug!("\tgather_lms: found suffix {}", i+1);
            }
            false
        }
    });

    if succ_t {
        *iter.next().unwrap() = 0;
        debug!("\tgather_lms: found first suffix");
    }
    assert!(iter.next().is_none());
}

fn gather_lms_post(sa_new: &mut [Suffix], input_new: &mut [Suffix]) {
    for suf in sa_new.iter_mut() {
        *suf = input_new[*suf as usize];
    }
}

fn put_suffix<T: ToPrimitive>(suffixes: &mut [Suffix], n1: usize, input: &[T], buckets: &mut [Suffix]) {
    // Find the end of each bucket.
    get_buckets(input, buckets, true);

    //TEMP: copy suffixes to the beginning of the list
    for i in 0 .. n1 {
        suffixes[i] = suffixes[n1 + i];
    }
    fill(&mut suffixes[n1..], SUF_INVALID);

    debug!("put_suffix prelude: {:?}", suffixes);

    for i in (0 .. n1).rev() {
        let p = suffixes[i];
        assert!(p != SUF_INVALID);
        suffixes[i] = SUF_INVALID;
        let sym = &input[p as usize];
        let buck = &mut buckets[sym.to_usize().unwrap()];
        *buck -= 1;
        assert!(*buck as usize >= i);
        suffixes[*buck as usize] = p;
    }

    debug!("put_suffix: {:?}", suffixes);
}


fn saca<T: Eq + Ord + ToPrimitive>(input: &[T], alphabet_size: usize, storage: &mut [Suffix]) {
    debug!("saca: entry");
    assert!(input.len() + alphabet_size <= storage.len());

    // Stage 1: reduce the problem by at least 1/2.
    let n1 = {
        let (suffixes, rest) = storage.split_at_mut(input.len());
        let excess = rest.len();
        debug!("input len: {}, excess: {}", input.len(), excess);
        let (_,buckets) = rest.split_at_mut(excess - alphabet_size);
        put_substr(suffixes, input, buckets);
        induce_low(suffixes, input, buckets, true);
        induce_sup(suffixes, input, buckets, true);

        // Now, all the LMS-substrings are sorted and stored sparsely in SA.
        // Compact all the sorted substrings into the first n1 items of SA.
        let mut lms_count = 0usize;
        for i in 0 .. suffixes.len() {
            if suffixes[i] != SUF_INVALID {
                suffixes[lms_count] = suffixes[i];
                lms_count += 1;
            }
        }

        debug!("Compacted LMS: {:?}", &suffixes[..lms_count]);
        lms_count
    };

    // Stage 2: solve the reduced problem.
    {
        assert!(n1+n1 <= input.len());
        let (input_new, sa_new) = storage.split_at_mut(n1);
        let num_names = name_substr(&mut sa_new[.. input.len()/2], input_new, input);
        debug!("named input_new: {:?}", input_new);
        debug!("num_names = {}", num_names);
        assert!(num_names <= n1);

        if num_names < n1 {
            // Recurse if names are not yet unique.
            saca(input_new, num_names, sa_new);
        }else {
            // Get the suffix array of s1 directly.
            for (i,&sym) in input_new.iter().enumerate() {
                sa_new[sym as usize] = i as Suffix;
            }
            debug!("Sorted suffixes: {:?}", &sa_new[..n1]);
        }

        let slice = &mut sa_new[..n1];
        gather_lms(input_new, input);
        gather_lms_post(slice, input_new);
        debug!("Gathered LMS: {:?}", slice);
    }

    // Stage 3: induce SA(S) from SA(S1).
    {
        let (suffixes, rest) = storage.split_at_mut(input.len());
        let excess = rest.len();
        let (_, buckets) = rest.split_at_mut(excess - alphabet_size);
        put_suffix(suffixes, n1, input, buckets);
        induce_low(suffixes, input, buckets, false);
        induce_sup(suffixes, input, buckets, false);

        if log_enabled!(LogLevel::Debug) {
            for (i,p) in suffixes.iter().enumerate() {
                assert_eq!(suffixes[..i].iter().find(|suf| *suf==p), None);
                assert!(i == 0 || input[suffixes[i-1] as usize] <= input[suffixes[i] as usize]);
            }
        }
    }
}


/// Suffix Array Constructor
pub struct Constructor {
    suffixes    : Vec<Suffix>,
    n           : usize,
}

impl Constructor {
    /// Create a new instance for a given maximum input size
    pub fn new(max_n: usize) -> Constructor {
        use std::{cmp, iter};
        let extra_2s = (1usize<<15) + (1usize<<7);
        let extra = 0x100 + cmp::max(max_n/4, cmp::min(extra_2s, max_n/2));
        info!("n: {}, extra words: {}", max_n, extra);
        Constructor {
            suffixes: iter::repeat(0 as Suffix).take(max_n+extra).collect(),
            n       : max_n,
        }
    }

    /// Return maximum block size
    pub fn capacity(&self) -> usize {
        self.n
    }

    /// Compute the suffix array for a given input
    pub fn compute<'a>(&'a mut self, input: &[Symbol]) -> &'a [Suffix] {
        assert_eq!(input.len(), self.n);
        if true {
            saca(input, 0x100, &mut self.suffixes[..]);
        }else {
            sort_direct(input, &mut self.suffixes[..]);
        }

        debug!("construct suf: {:?}", &self.suffixes[.. self.n]);
        &self.suffixes[.. self.n]
    }

    /// Temporarily provide the storage for outside needs
    pub fn reuse<'a>(&'a mut self) -> &'a mut [Suffix] {
        &mut self.suffixes[..]
    }
}


#[cfg(test)]
pub mod test {
    #[cfg(feature="unstable")]
    use test::Bencher;
    use compress::bwt;

    fn some_detail(input: &[super::Symbol], suf_expected: &[super::Suffix], origin_expected: usize, out_expected: &[super::Symbol]) {
        let mut con = super::Constructor::new(input.len());
        let (output, origin) = {
            let suf = con.compute(input);
            assert_eq!(&suf[..], suf_expected);
            let mut iter = bwt::TransformIterator::new(input, suf);
            let out: Vec<super::Symbol> = iter.by_ref().collect();
            (out, iter.get_origin())
        };
        assert_eq!(origin, origin_expected);
        assert_eq!(&output[..], out_expected);
        let suf = &mut con.reuse()[.. input.len()];
        let decoded: Vec<super::Symbol> = bwt::decode(&output, origin, suf).collect();
        assert_eq!(&input[..], &decoded[..]);
    }

    #[test]
    fn detailed() {
        some_detail(b"abracadabra", &[10,7,0,3,5,8,1,4,6,9,2], 2, b"rdarcaaaabb");
        some_detail(b"banana", &[5,3,1,0,4,2], 3, b"nnbaaa");
    }

    fn some_roundtrip(input: &[super::Symbol]) {
        let mut con = super::Constructor::new(input.len());
        let (output, origin) = {
            let suf = con.compute(input);
            let mut iter = bwt::TransformIterator::new(input, suf);
            let out: Vec<super::Symbol> = iter.by_ref().collect();
            (out, iter.get_origin())
        };
        let decoded: Vec<super::Symbol> =
            bwt::decode(&output, origin, &mut con.reuse()[.. input.len()]).
            take(input.len()).collect();
        assert_eq!(&input[..], &decoded[..]);
    }

    #[test]
    fn roundtrips() {
        some_roundtrip(include_bytes!("../LICENSE"));
        //some_roundtrip(include_bin!("../bin/dark"));
    }

    #[cfg(feature="unstable")]
    #[bench]
    fn speed(bh: &mut Bencher) {
        let input = include_bytes!("../LICENSE");
        let mut con = super::Constructor::new(input.len());
        bh.iter(|| {
            con.compute(input);
        });
        bh.bytes = input.len() as u64;
    }
}
