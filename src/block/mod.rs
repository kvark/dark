/*!

Block encoding/decoding
*/

use std::io;
use compress::entropy::ari;

/// DC based
pub mod dc;
/// Raw
pub mod raw;


#[cfg(feature="tune")]
fn print_stats<W: io::Write>(eh: &ari::Encoder<W>) {
    let (b0, b1) = eh.get_bytes_lost();
    info!("Bytes lost on threshold cut: {}, on divisions: {}", b0, b1);
}

#[cfg(not(feature="tune"))]
fn print_stats<W: io::Write>(_eh: &ari::Encoder<W>) {
    //empty
}

/// Generic block encoder
pub trait Encoder {
	/// Encode a block into a given writer
	fn encode<W: io::Write>(&mut self, &[u8], W) -> (W, io::Result<()>);
}

/// Generic block decoder
pub trait Decoder {
	/// Decode a block by reading from a given Reader into some Writer
	fn decode<R: io::Read, W: io::Write>(&mut self, R, W) -> (R, W, io::Result<()>);
}
