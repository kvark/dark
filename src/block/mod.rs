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
