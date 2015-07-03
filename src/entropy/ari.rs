#![allow(missing_docs)] //temp

use compress::entropy::ari::apm;

pub type Border = u32;
pub type Bit = u8;

pub struct Range {
    lo: Border,
    hi: Border,
}

impl Range {
    pub fn new() -> Range {
        Range {
            lo: 0,
            hi: !0,
        }
    }

    #[inline]
    fn get_mid(&self, model: apm::Bit) -> Border {
        let flat = (model.to_flat() + 1 -
            (model.to_flat() >> (apm::FLAT_BITS - 1))
            ) as Border;
        let diff = self.hi - self.lo;
        let flat_mask = apm::FLAT_TOTAL as Border - 1;
        self.lo + (diff >> apm::FLAT_BITS) * flat +
            (((diff & flat_mask) * flat) >> apm::FLAT_BITS)
    }

    #[inline]
    fn roll(&mut self, out: &mut [u8]) -> usize {
        let mut count = 0;
        while (self.lo ^ self.hi) & 0xFF000000 == 0 {
            out[count] = (self.lo >> 24) as u8;
            count += 1;
            self.lo <<= 8;
            self.hi = (self.hi << 8) | 0xFF;
        }
        count
    }

    pub fn encode(&mut self, bit: Bit, model: apm::Bit, out: &mut [u8]) -> usize {
        let mid = self.get_mid(model);
        debug_assert!(self.lo <= mid && mid < self.hi);
        if bit == 0 {
            self.hi = mid;
        }else {
            self.lo = mid + 1;
        }
        self.roll(out)
    }

    pub fn decode(&mut self, value: Border, model: apm::Bit) -> (Bit, usize) {
        let mid = self.get_mid(model);
        let mut temp = [0u8; 4];
        if value <= mid {
            self.hi = mid;
            (0, self.roll(&mut temp))
        }else {
            self.lo = mid + 1;
            (1, self.roll(&mut temp))
        }
    }

    pub fn post_encode(&self, out: &mut [u8]) -> usize {
        // Flush first unequal byte of range
        for i in (0..4) {
            out[i] = (self.lo >> (24 - i*8)) as u8;
        }
        4
    }
}

#[test]
fn roundtrip() {
    let data = [1u8, 84, 15, 91];
    let mut ptr = 0usize;
    let mut buffer = [0u8; 100];
    let mut range = Range::new();
    let apm = apm::Bit::new_equal();
    for d in data.iter() {
        for i in 0..8 {
            let bit = ((*d >> i) & 1) as Bit;
            let count = range.encode(bit, apm, &mut buffer[ptr..]);
            ptr += count;
        }
    }
    let total = ptr + range.post_encode(&mut buffer[ptr..]);
    let mut undata = [0u8; 4];
    range = Range::new();
    ptr = 4;
    let mut value = buffer[0..4].iter().fold(0, |u, b| (u<<8) + (*b as Border));
    for d in undata.iter_mut() {
        for i in 0..8 {
            let (bit, count) = range.decode(value, apm);
            *d |= (bit as u8) << i;
            for _ in 0 .. count {
                debug_assert!(ptr < total);
                value = (value << 8) | (buffer[ptr] as Border);
                ptr += 1;
            }
        }
    }
    debug_assert_eq!(data, undata);
}
