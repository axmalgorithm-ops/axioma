pub mod entropy;
pub mod codec;
pub use crate::codec::{compress, decompress};
pub use crate::entropy::{FastAdaptiveModel, FastRangeDecoder, FastRangeEncoder};
