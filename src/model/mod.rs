/*!

Various BWT-DC compression models

*/

use byteorder::{LittleEndian, WriteBytesExt};
use compress::entropy::ari;
pub use compress::bwt::dc::Context;
use std::io;
use std::fs::File;

/// Old Dark-0.51 model
pub mod dark;
/// Original BWT-DC compression model
pub mod exp;
/// A simplest model to compare with
pub mod simple;
/// A attempt to reproduce YBS model
pub mod ybs;

/// Distance type
pub type Distance = u32;
/// Symbol type
pub type Symbol = u8;


/// A generic BWT-DC output coding model
pub trait DistanceModel {
    /// Create a new default instance
    fn new_default() -> Self;
    /// Reset current estimations
    fn reset(&mut self);
    //TODO: return Result
    /// Encode a distance for some symbol
    fn encode<W: io::Write>(&mut self, Distance, &Context, &mut ari::Encoder<W>);
    //TODO: return Result
    /// Decode a distance for some symbol
    fn decode<R: io::Read>(&mut self, &Context, &mut ari::Decoder<R>) -> Distance;
}


/// Raw (Sym,Dist) pairs output
pub struct RawOut {
    out: File,
}

impl DistanceModel for RawOut {
    fn new_default() -> RawOut {
        RawOut {
            out: File::create("out.raw").unwrap(),
        }
    }

    fn reset(&mut self) {}

    fn encode<W: io::Write>(&mut self, d: Distance, c: &Context, _enc: &mut ari::Encoder<W>) {
        debug!("Encoding raw distance {} for symbol {}", d, c.symbol);
        self.out.write_u32::<LittleEndian>(d).and(
            self.out.write_u8(c.symbol)).and(
            self.out.write_u8(c.last_rank)).and(
            self.out.write_u32::<LittleEndian>(c.distance_limit as u32)).unwrap();
    }

    fn decode<R: io::Read>(&mut self, _c: &Context, _dec: &mut ari::Decoder<R>) -> Distance {
        0   //not supported
    }
}


#[cfg(test)]
pub mod test {
    use std::io;
    use rand::{Rng, StdRng};
    use compress::entropy::ari;
    use super::{Context, Distance, DistanceModel};
    use super::Symbol;

    fn roundtrip<M: DistanceModel>(input: &[(Distance,Context)]) {
        let mut m: M = DistanceModel::new_default();
        let mut eh = ari::Encoder::new(Vec::new());
        m.reset();
        for &(dist, ref ctx) in input.iter() {
            debug!("Encode: {}", dist);
            m.encode(dist, ctx, &mut eh);
        }
        let (mem, err) = eh.finish();
        err.unwrap();
        m.reset();
        let mut dh = ari::Decoder::new(io::BufReader::new(io::Cursor::new(&mem[..])));
        for &(dist, ref ctx) in input.iter() {
            let d2 = m.decode(ctx, &mut dh);
            debug!("Actual: {}, Decoded: {}", dist, d2);
            assert_eq!(d2, dist);
        }
    }

    fn gen_data(size: usize, max_dist: Distance) -> Vec<(Distance,Context)> {
        let mut rng = StdRng::new().unwrap();
        (0..size).map(|_| {
            let sym: Symbol = rng.gen();
            let ctx = Context::new(sym, 0, max_dist as usize);
            (rng.gen_range(0, max_dist), ctx)
        }).collect()
    }

    fn roundtrips<M: DistanceModel>() {
        roundtrip::<M>(&[
            (1, Context::new(1,1,5)),
            (2, Context::new(2,2,5)),
            (3, Context::new(3,3,5)),
            (4, Context::new(4,4,5))
            ]);
        roundtrip::<M>(&gen_data(1000,200));
    }
    
    #[test]
    fn roundtrips_dark() {
        roundtrips::<super::dark::Model>();
    }

    #[test]
    fn roundtrips_exp() {
        roundtrips::<super::exp::Model>();
    }

    #[test]
    fn roundtrips_simple() {
        roundtrips::<super::simple::Model>();
    }

    #[test]
    fn roundtrips_ybs() {
        roundtrips::<super::ybs::Model>();
    }
}
