/*!

Various BWT-DC compression models

*/

use byteorder::{LittleEndian, WriteBytesExt};
use compress::bwt::dc;
use compress::entropy::ari;
use std::io;
use std::fs::File;

/// A copy of `bbb` model
pub mod bbb;
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
/// Symbol encoding context //TODO
pub type SymContext = ();

/// An abstract BWT output encoding model (BWT-???-Ari)
pub trait Model<T, C> {
    /// Reset current estimations
    fn reset(&mut self);
    /// Encode an element
    fn encode<W: io::Write>(&mut self, T, &C, &mut ari::Encoder<W>) -> io::Result<()>;
    /// Decode an element
    fn decode<R: io::Read>(&mut self, &C, &mut ari::Decoder<R>) -> io::Result<T>;
}

/// A generic BWT-DC output coding model
pub trait DistanceModel: Model<Distance, dc::Context> {}
impl<M: Model<Distance, dc::Context>> DistanceModel for M {}

/// A generic BWT raw output coding model
pub trait RawModel: Model<Symbol, SymContext> {}
impl<M: Model<Symbol, SymContext>> RawModel for M {}

/// Raw (Sym,Dist) pairs output
pub struct RawOut {
    out: File,
}

impl RawOut {
    /// Create a new raw output model
    pub fn new() -> RawOut {
        RawOut {
            out: File::create("out.raw").unwrap(),
        }
    }
}

impl Model<Distance, dc::Context> for RawOut {
    fn reset(&mut self) {}

    fn encode<W: io::Write>(&mut self, d: Distance, c: &dc::Context, _enc: &mut ari::Encoder<W>) -> io::Result<()> {
        debug!("Encoding raw distance {} for symbol {}", d, c.symbol);
        try!(self.out.write_u32::<LittleEndian>(d));
        try!(self.out.write_u8(c.symbol));
        try!(self.out.write_u8(c.last_rank));
        try!(self.out.write_u32::<LittleEndian>(c.distance_limit as u32));
        Ok(())
    }

    fn decode<R: io::Read>(&mut self, _c: &dc::Context, _dec: &mut ari::Decoder<R>) -> io::Result<Distance> {
        Ok(0) //not supported
    }
}


#[cfg(test)]
pub mod test {
    use std::io;
    use rand::{Rng, StdRng};
    use compress::bwt::dc;
    use compress::entropy::ari;
    use super::{Distance, DistanceModel};
    use super::Symbol;

    fn roundtrip<M: DistanceModel>(m: &mut M, input: &[(Distance, dc::Context)]) {
        let mut eh = ari::Encoder::new(Vec::new());
        m.reset();
        for &(dist, ref ctx) in input.iter() {
            debug!("Encode: {}", dist);
            m.encode(dist, ctx, &mut eh).unwrap();
        }
        let (mem, err) = eh.finish();
        err.unwrap();
        m.reset();
        let mut dh = ari::Decoder::new(io::BufReader::new(io::Cursor::new(&mem[..])));
        for &(dist, ref ctx) in input.iter() {
            let d2 = m.decode(ctx, &mut dh).unwrap();
            debug!("Actual: {}, Decoded: {}", dist, d2);
            assert_eq!(d2, dist);
        }
    }

    fn gen_data(size: usize, max_dist: Distance) -> Vec<(Distance, dc::Context)> {
        let mut rng = StdRng::new().unwrap();
        (0..size).map(|_| {
            let sym: Symbol = rng.gen();
            let ctx = dc::Context::new(sym, 0, max_dist as usize);
            (rng.gen_range(0, max_dist), ctx)
        }).collect()
    }

    fn roundtrips<M: DistanceModel>(mut m: M) {
        roundtrip(&mut m, &[
            (1, dc::Context::new(1,1,5)),
            (2, dc::Context::new(2,2,5)),
            (3, dc::Context::new(3,3,5)),
            (4, dc::Context::new(4,4,5))
            ]);
        roundtrip(&mut m, &gen_data(1000,200));
    }
    
    #[test]
    fn roundtrips_dark() {
        roundtrips(super::dark::Model::new());
    }

    #[test]
    fn roundtrips_exp() {
        roundtrips(super::exp::Model::new());
    }

    #[test]
    fn roundtrips_simple() {
        roundtrips(super::simple::Model::new());
    }

    #[test]
    fn roundtrips_ybs() {
        roundtrips(super::ybs::Model::new());
    }
}
