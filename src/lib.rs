#![no_std]
extern crate alloc;
pub mod preprocessor;
pub mod error;

pub use preprocessor::{Preprocessor, NoOpPreprocessor, DeltaPreprocessor, TextPreprocessor, Lz77Preprocessor};

// memory profile trait to squeeze buffer state on tiny mcus like cortex-m0
pub trait CompressionConfig {
    const STATE_BUFFER_SIZE: usize;
    const CONTEXT_TABLE_SIZE: usize;
}

// default beast mode for desktop or beefy arm cores
pub struct StandardConfig;
impl CompressionConfig for StandardConfig {
    const STATE_BUFFER_SIZE: usize = 65536; // 64kb state buffer
    const CONTEXT_TABLE_SIZE: usize = 65536;
}

// tight profile for tiny chips with very low ram (<32kb total)
pub struct CortexM0Config;
impl CompressionConfig for CortexM0Config {
    const STATE_BUFFER_SIZE: usize = 4096; // 4kb to fit tiny ram
    const CONTEXT_TABLE_SIZE: usize = 4096;
}
pub mod context;
pub mod decoder;
pub mod encoder;
pub mod range_coder;
pub use encoder::Encoder;
pub use decoder::Decoder;
