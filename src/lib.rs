pub mod codec;
pub mod entropy;

pub use entropy::FastAdaptiveModel;
pub use codec::{FastRangeEncoder, FastRangeDecoder};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_text() {
        let text = b"Hello, Axioma compression algorithm!";
        let mut encoder = FastRangeEncoder::with_capacity(text.len());
        let mut model_enc = FastAdaptiveModel::default();

        for &byte in text {
            encoder.encode(byte, &mut model_enc);
        }
        let compressed = encoder.finish();

        let mut decoder = FastRangeDecoder::new(compressed).expect("Failed to init decoder");
        let mut model_dec = FastAdaptiveModel::default();
        let mut decompressed = Vec::with_capacity(text.len());

        for _ in 0..text.len() {
            let byte = decoder.decode(&mut model_dec);
            decompressed.push(byte);
        }

        assert_eq!(text.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_roundtrip_random() {
        let mut data = vec![0u8; 1000];
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = ((i * 37 + 13) % 256) as u8;
        }

        let mut encoder = FastRangeEncoder::with_capacity(data.len());
        let mut model_enc = FastAdaptiveModel::default();

        for &byte in &data {
            encoder.encode(byte, &mut model_enc);
        }
        let compressed = encoder.finish();

        let mut decoder = FastRangeDecoder::new(compressed).expect("Failed to init decoder");
        let mut model_dec = FastAdaptiveModel::default();
        let mut decompressed = Vec::with_capacity(data.len());

        for _ in 0..data.len() {
            let byte = decoder.decode(&mut model_dec);
            decompressed.push(byte);
        }

        assert_eq!(data.as_slice(), decompressed.as_slice());
    }
}
