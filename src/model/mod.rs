/*!

Various BWT-DC compression models

*/

use compress::bwt::dc;
use compress::entropy::ari;
use std::io;

/// A copy of `bbb` model
pub mod bbb;
/// Old Dark-0.51 model
pub mod dark;
/// Original BWT-DC compression model
pub mod exp;
/// Raw output for debugging
pub mod raw;
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


#[cfg(test)]
pub mod test {
    use std::io;
    use rand::{Rng, StdRng};
    use compress::bwt::dc;
    use compress::entropy::ari;
    use super::{Distance, DistanceModel};
    use super::{RawModel, Symbol, SymContext};

    fn roundtrip_dc<M: DistanceModel>(m: &mut M, input: &[(Distance, dc::Context)]) {
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

    fn roundtrip_raw<M: RawModel>(mut m: M, input: &[(Symbol, SymContext)]) {
        let mut eh = ari::Encoder::new(Vec::new());
        m.reset();
        for &(sym, ref ctx) in input.iter() {
            debug!("Encode: {}", sym);
            m.encode(sym, ctx, &mut eh).unwrap();
        }
        let (mem, err) = eh.finish();
        err.unwrap();
        m.reset();
        let mut dh = ari::Decoder::new(io::BufReader::new(io::Cursor::new(&mem[..])));
        for &(sym, ref ctx) in input.iter() {
            let sym2 = m.decode(ctx, &mut dh).unwrap();
            debug!("Actual: {}, Decoded: {}", sym, sym2);
            assert_eq!(sym, sym2);
        }
    }

    fn gen_data_dc(size: usize, max_dist: Distance) -> Vec<(Distance, dc::Context)> {
        let mut rng = StdRng::new().unwrap();
        (0..size).map(|_| {
            let sym: Symbol = rng.gen();
            let ctx = dc::Context::new(sym, 0, max_dist as usize);
            (rng.gen_range(0, max_dist), ctx)
        }).collect()
    }

    fn gen_data_raw(size: usize) -> Vec<(Symbol, SymContext)> {
        let mut rng = StdRng::new().unwrap();
        (0..size).map(|_| {
            let sym: Symbol = rng.gen();
            (sym, ())
        }).collect()
    }

    fn roundtrips_dc<M: DistanceModel>(mut m: M) {
        roundtrip_dc(&mut m, &[
            (1, dc::Context::new(1,1,5)),
            (2, dc::Context::new(2,2,5)),
            (3, dc::Context::new(3,3,5)),
            (4, dc::Context::new(4,4,5))
            ]);
        roundtrip_dc(&mut m, &gen_data_dc(1000,200));
    }

    #[test]
    fn roundtrip_bbb() {
        let input = gen_data_raw(1000);
        roundtrip_raw(super::bbb::Model::new(), &input);
    }
    
    #[test]
    fn roundtrips_dark() {
        roundtrips_dc(super::dark::Model::new());
    }

    #[test]
    fn roundtrips_exp() {
        roundtrips_dc(super::exp::Model::new());
    }

    #[test]
    fn roundtrips_simple() {
        roundtrips_dc(super::simple::Model::new());
    }

    #[test]
    fn roundtrips_ybs() {
        roundtrips_dc(super::ybs::Model::new());
    }
}
