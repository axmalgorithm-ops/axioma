use crate::entropy::{FastAdaptiveModel, FastRangeEncoder};
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut model = FastAdaptiveModel::new();
    let mut encoder = FastRangeEncoder::new();
    for &byte in input {
        for i in 0..8 { encoder.encode((byte >> i) & 1 == 1, &mut model); }
    }
    encoder.finish()
}
pub fn decompress(input: &[u8]) -> Result<Vec<u8>, String> {
    Ok(input.to_vec())
}
