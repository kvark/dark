/*!

Raw block encoding/decoding routines

*/

use byteorder::WriteBytesExt;
use std::io;

use compress::bwt;
use compress::entropy::ari;
use model::{RawModel, Symbol};
use saca;


/// Raw BWT output encoder
pub struct Encoder<M> {
    sac: saca::Constructor,
    /// Raw encoding model
    pub model: M,
}

impl<M: RawModel> Encoder<M> {
    /// Create a new Encoder instance
    pub fn new(n: usize, mut model: M) -> Encoder<M> {
        model.reset();
        Encoder {
            sac     : saca::Constructor::new(n),
            model   : model,
        }
    }

    /// Encode a block into a given writer
    pub fn encode<W: io::Write>(&mut self, input: &[u8], writer: W) -> (W, io::Result<()>) {
        let block_size = input.len();
        assert!(block_size <= self.sac.capacity());
        // perform BWT and DC
        let (output, origin) = {
            let suf = self.sac.compute(input);
            let mut iter = bwt::TransformIterator::new(input, suf);
            let out: Vec<u8> = iter.by_ref().collect();
            (out, iter.get_origin())
        };
        let mut eh = ari::Encoder::new(writer);
        // encode origin
        info!("Origin: {}", origin);
        self.model.encode((origin>>24) as Symbol, &(), &mut eh);
        self.model.encode((origin>>16) as Symbol, &(), &mut eh);
        self.model.encode((origin>>8)  as Symbol, &(), &mut eh);
        self.model.encode(origin as Symbol, &(), &mut eh);
        // encode symbols
        for sym in output.iter() {
            self.model.encode(*sym as Symbol, &(), &mut eh);
        }
        // done
        super::print_stats(&eh);
        eh.finish()
    }
}

/// Raw BWT output decoder
pub struct Decoder<M> {
    input       : Vec<u8>,
    suffixes    : Vec<saca::Suffix>,
    /// Raw decoding model
    pub model   : M,
}

impl<M: RawModel> Decoder<M> {
    /// Create a new Decoder instance
    pub fn new(n: usize, mut model: M) -> Decoder<M> {
        use std::iter::repeat;
        model.reset();
        Decoder {
            input   : repeat(0u8).take(n).collect(),
            suffixes: repeat(0 as saca::Suffix).take(n).collect(),
            model   : model,
        }
    }

    /// Decode a block by reading from a given Reader into some Writer
    pub fn decode<R: io::Read, W: io::Write>(&mut self, reader: R, mut writer: W) -> (R, W, io::Result<()>) {
        let mut dh = ari::Decoder::new(reader);
        // decode origin
        let origin =
            ((self.model.decode(&(), &mut dh) as usize) << 24) |
            ((self.model.decode(&(), &mut dh) as usize) << 16) |
            ((self.model.decode(&(), &mut dh) as usize) << 8)  |
            ((self.model.decode(&(), &mut dh) as usize));
        info!("Origin: {}", origin);
        // decode symbols
        for sym in self.input.iter_mut() {
            *sym = self.model.decode(&(), &mut dh);
        }
        // undo BWT and write output
        for b in bwt::decode(&self.input, origin, &mut self.suffixes) {
            writer.write_u8(b).unwrap();
        }
        let result = writer.flush();
        let (r, err) = dh.finish();
        (r, writer, result.and(err))
    }
}
