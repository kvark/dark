pub mod ari;

use byteorder::ReadBytesExt;
use compress::entropy::ari::apm;
use std::io;

const BORDER_BYTES: usize = 4;


/// An arithmetic encoder helper
pub struct Encoder<W> {
    stream: W,
    range: ari::Range,
}

impl<W: io::Write> Encoder<W> {
    /// Create a new encoder on top of a given Writer
    pub fn new(w: W) -> Encoder<W> {
        Encoder {
            stream: w,
            range: ari::Range::new(),
        }
    }

    /// Encode a bit
    pub fn encode(&mut self, bit: ari::Bit, model: apm::Bit) -> io::Result<()> {
        let mut buf = [0u8; 4];
        let num = self.range.encode(bit, model, &mut buf[..]);
        self.stream.write(&buf[..num]).map(|_| ())
    }

    /// Finish encoding by writing the code tail word
    pub fn finish(mut self) -> (W, io::Result<()>) {
    	let mut buf = [0u8; BORDER_BYTES];
    	let num = self.range.post_encode(&mut buf);
    	let mut result = self.stream.write(&buf[..num]).map(|_| ());
        result = result.and(self.stream.flush());
        (self.stream, result)
    }
}

/// An arithmetic decoder helper
pub struct Decoder<R> {
    stream: R,
    range: ari::Range,
    code: ari::Border,
    bytes_pending: usize,
}

impl<R: io::Read> Decoder<R> {
    /// Create a decoder on top of a given Reader
    pub fn new(r: R) -> Decoder<R> {
        Decoder {
            stream: r,
            range: ari::Range::new(),
            code: 0,
            bytes_pending: BORDER_BYTES,
        }
    }

    fn feed(&mut self) -> io::Result<()> {
        for _ in 0 .. self.bytes_pending {
            let b = try!(self.stream.read_u8());
            self.code = (self.code<<8) + (b as ari::Border);
        }
        self.bytes_pending = 0;
        Ok(())
    }

    /// Decode a bit
    pub fn decode(&mut self, model: apm::Bit) -> io::Result<ari::Bit> {
        try!(self.feed());
        let (value, num) = self.range.decode(self.code, model);
        self.bytes_pending = num;
        Ok(value)
    }

    /// Finish decoding
    pub fn finish(mut self) -> (R, io::Result<()>)  {
        let result = self.feed();
        (self.stream, result)
    }
}
