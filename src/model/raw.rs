use byteorder::{LittleEndian, WriteBytesExt};
use compress::bwt::dc;
use compress::entropy::ari;
use std::fs::File;
use std::io;

use super::{Distance, Model};
use super::{Symbol, SymContext};


/// Raw (Sym, Dist) pairs output
pub struct DcOut {
    out: File,
}

impl DcOut {
    /// Create a new raw output model
    pub fn new() -> DcOut {
        DcOut {
            out: File::create("out-dc.raw").unwrap(),
        }
    }
}

impl Model<Distance, dc::Context> for DcOut {
    fn reset(&mut self) {}

    fn encode<W: io::Write>(&mut self, d: Distance, c: &dc::Context,
              _enc: &mut ari::Encoder<W>) -> io::Result<()>
    {
        debug!("Encoding raw distance {} for symbol {}", d, c.symbol);
        try!(self.out.write_u32::<LittleEndian>(d));
        try!(self.out.write_u8(c.symbol));
        try!(self.out.write_u8(c.last_rank));
        try!(self.out.write_u32::<LittleEndian>(c.distance_limit as u32));
        Ok(())
    }

    fn decode<R: io::Read>(&mut self, _c: &dc::Context,
              _dec: &mut ari::Decoder<R>) -> io::Result<Distance>
    {
        Ok(0) //not supported
    }
}

/// Raw sym output
pub struct Out {
    out: File,
}

impl Out {
    /// Create a new raw output model
    pub fn new() -> Out {
        Out {
            out: File::create("out.raw").unwrap(),
        }
    }
}

impl Model<Symbol, SymContext> for Out {
    fn reset(&mut self) {}

    fn encode<W: io::Write>(&mut self, sym: Symbol, _: &SymContext,
              _enc: &mut ari::Encoder<W>) -> io::Result<()>
    {
        debug!("Encoding raw symbol {}", sym);
        try!(self.out.write_u8(sym));
        Ok(())
    }

    fn decode<R: io::Read>(&mut self, _c: &SymContext,
              _dec: &mut ari::Decoder<R>) -> io::Result<Symbol>
    {
        Ok(0) //not supported
    }
}
