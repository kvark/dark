/*!

DC-based block encoding/decoding routines

*/

use byteorder::WriteBytesExt;
use std::io;

use compress::bwt;
use compress::entropy::ari;
use model::{Distance, DistanceModel};
use saca;


const CTX_0: bwt::dc::Context = bwt::dc::Context {
    symbol: 0, last_rank: 0, distance_limit: 0x101,
};

/// A basic block encoder
pub struct Encoder<M> {
    sac: saca::Constructor,
    mtf: bwt::mtf::MTF,
    /// Distance encoding model
    pub model: M,
}

impl<M: DistanceModel> Encoder<M> {
    /// Create a new Encoder instance
    pub fn new(n: usize, mut model: M) -> Encoder<M> {
        model.reset();
        Encoder {
            sac     : saca::Constructor::new(n),
            mtf     : bwt::mtf::MTF::new(),
            model   : model,
        }
    }
}

impl<M: DistanceModel> super::Encoder for Encoder<M> {
    fn encode<W: io::Write>(&mut self, input: &[u8], writer: W) -> (W, io::Result<()>) {
        let block_size = input.len();
        assert!(block_size <= self.sac.capacity());
        // perform BWT and DC
        let (output, origin) = {
            let suf = self.sac.compute(input);
            let mut iter = bwt::TransformIterator::new(input, suf);
            let out: Vec<u8> = iter.by_ref().collect();
            (out, iter.get_origin())
        };
        let suf = &mut self.sac.reuse()[.. block_size];
        let dc_iter = bwt::dc::encode(&output, suf, &mut self.mtf);
        let mut eh = ari::Encoder::new(writer);
        {   // encode init distances
            let mut cur_active = true;
            let mut i = 0usize;
            while i<0xFF {
                let base = i;
                if cur_active {
                    while i<0xFF && dc_iter.get_init()[i]<block_size {
                        i += 1;
                    }
                    let num = (if base==0 {i} else {i-base-1}) as Distance;
                    debug!("Init fill num {}", num);
                    self.model.encode(num, &CTX_0, &mut eh);
                    for (sym,d) in dc_iter.get_init().iter().enumerate().skip(base).take(i-base) {
                        let ctx = bwt::dc::Context::new(sym as u8, 0, input.len());
                        self.model.encode(*d as Distance, &ctx, &mut eh);
                        debug!("Init {} for {}", *d, sym);
                    }
                    cur_active = false;
                }else {
                    while {i+=1; i<0xFF && dc_iter.get_init()[i] == block_size} {}
                    let num = (i-base-1) as Distance;
                    debug!("Init empty num {}", num);
                    self.model.encode(num, &CTX_0, &mut eh);
                    cur_active = true;
                }
            }
        }
        // encode distances
        for (d,ctx) in dc_iter {
            debug!("Distance {} for {}", d, ctx.symbol);
            self.model.encode(d, &ctx, &mut eh);
        }
        // done
        info!("Origin: {}", origin);
        self.model.encode(origin as Distance, &CTX_0, &mut eh);
        super::print_stats(&eh);
        eh.finish()
    }
}


/// A basic block decoder
pub struct Decoder<M> {
    input       : Vec<u8>,
    suffixes    : Vec<saca::Suffix>,
    mtf         : bwt::mtf::MTF,
    /// Distance decoding model
    pub model   : M,
}

impl<M: DistanceModel> Decoder<M> {
    /// Create a new Decoder instance
    pub fn new(n: usize, mut model: M) -> Decoder<M> {
        use std::iter::repeat;
        model.reset();
        Decoder {
            input   : repeat(0u8).take(n).collect(),
            suffixes: repeat(0 as saca::Suffix).take(n).collect(),
            mtf     : bwt::mtf::MTF::new(),
            model   : model,
        }
    }
}

impl<M: DistanceModel> super::Decoder for Decoder<M> {
    fn decode<R: io::Read, W: io::Write>(&mut self, reader: R, mut writer: W) -> (R, W, io::Result<()>) {
        let model = &mut self.model;
        let mut dh = ari::Decoder::new(reader);
        // decode init distances
        let init = {
            let mut init = [self.input.len(); 0x100];
            let mut cur_active = true;
            let mut i = 0usize;
            while i<0xFF {
                let add  = if i==0 && cur_active {0usize} else {1usize};
                let num = model.decode(&CTX_0, &mut dh) as usize + add;
                debug!("Init num {}", num);
                if cur_active {
                    for (sym,d) in init.iter_mut().enumerate().skip(i).take(num)    {
                        let ctx = bwt::dc::Context::new(sym as u8, 0, self.input.len());
                        *d = model.decode(&ctx, &mut dh) as usize;
                        debug!("Init {} for {}", *d, sym);
                    }
                    cur_active = false;
                }else {
                    cur_active = true;
                }
                i += num;
            }
            init
        };
        // decode distances
        bwt::dc::decode(init, &mut self.input, &mut self.mtf, |ctx| {
            let d = model.decode(&ctx, &mut dh);
            debug!("Distance {} for {}", d, ctx.symbol);
            Ok(d as usize)
        }).unwrap();
        let origin = model.decode(&CTX_0, &mut dh) as usize;
        info!("Origin: {}", origin);
        // undo BWT and write output
        for b in bwt::decode(&self.input, origin, &mut self.suffixes) {
            writer.write_u8(b).unwrap();
        }
        let result = writer.flush();
        let (r, err) = dh.finish();
        (r, writer, result.and(err))
    }
}


#[cfg(test)]
pub mod test {
    use std::io;
    #[cfg(feature="unstable")]
    use std::iter::repeat;
    #[cfg(feature="unstable")]
    use test::Bencher;
    use model::{DistanceModel, exp, ybs};

    const TEXT: &'static [u8] = include_bytes!("../../LICENSE");

    fn roundtrip<M: DistanceModel>(model: M, bytes: &[u8]) {
        let mut enc = super::Encoder::new(bytes.len(), model);
        let (writer, err) = enc.encode(bytes, Vec::new());
        err.unwrap();
        let reader = io::BufReader::new(io::Cursor::new(&writer[..]));
        let mut dec = super::Decoder::new(bytes.len(), enc.model);
        let (_, output, err) = dec.decode(reader, Vec::new());
        err.unwrap();
        assert_eq!(&bytes[..], &output[..]);
    }

    #[test]
    fn roundtrips() {
        roundtrip(exp::Model::new(), b"abracababra");
        roundtrip(exp::Model::new(), TEXT);
        roundtrip(ybs::Model::new(), TEXT);
    }

    #[cfg(feature="unstable")]
    #[bench]
    fn encode_speed(bh: &mut Bencher) {
        let input = TEXT;
        let mut buffer: Vec<_> = repeat(0u8).take(input.len()).collect();
        let mut encoder = super::Encoder::new(input.len(), ybs::Model::new());
        bh.iter(|| {
            let (_, err) = encoder.encode(input, io::BufWriter::new(&mut buffer));
            err.unwrap();
        });
        bh.bytes = input.len() as u64;
    }

    #[cfg(feature="unstable")]
    #[bench]
    fn decode_speed(bh: &mut Bencher) {
        let input = TEXT;
        let mut encoder = super::Encoder::new(input.len(), ybs::Model::new());
        encoder.model.reset();
        let (writer, err) = encoder.encode(input, Vec::new());
        err.unwrap();
        let mut buffer: Vec<_> = repeat(0u8).take(input.len()).collect();
        let mut decoder = super::Decoder::new(input.len(), encoder.model);
        bh.iter(|| {
            decoder.model.reset();
            let (_, _, err) = decoder.decode(
                io::BufReader::new(io::Cursor::new(&writer[..])),
                io::BufWriter::new(&mut buffer));
            err.unwrap();
        });
        bh.bytes = input.len() as u64;
    }
}
