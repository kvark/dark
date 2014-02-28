/*!
	Various BWT-DC compression models
*/

use compress::entropy::ari;

/// Original BWT-DC compression model
pub mod dc;
/// A attempt to reproduce YBS model
pub mod ybs;

pub type Distance = u32;
pub type Symbol = u8;


/// A generic BWT-DC output coding model
pub trait DistanceModel {
	/// Reset current estimations
	fn reset(&mut self);
	/// Encode a distance for some symbol
	fn encode<W: Writer>(&mut self, Distance, Symbol, &mut ari::Encoder<W>);
	/// Decode a distance for some symbol
	fn decode<R: Reader>(&mut self, Symbol, &mut ari::Decoder<R>) -> Distance;
}

/// Create a default new coding model
pub fn new() -> dc::Model {
	dc::Model::new(ari::range_default_threshold >> 2)
}
