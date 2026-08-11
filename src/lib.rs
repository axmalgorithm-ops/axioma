#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod error;
pub mod preprocessor;
pub mod context;
pub mod range_coder;
pub mod encoder;
pub mod decoder;

pub use error::Error;
pub use preprocessor::{Preprocessor, NoOpPreprocessor, DeltaPreprocessor, TextPreprocessor};
pub use encoder::Encoder;
pub use decoder::Decoder;
