pub mod entropy;
pub mod codec;

use entropy::FastAdaptiveModel;
use codec::{FastRangeEncoder, FastRangeDecoder};

/// Сжимает массив байт.
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut model = FastAdaptiveModel::new();
    let mut enc = FastRangeEncoder::with_capacity(input.len() / 2 + 16);

    for &byte in input {
        enc.encode(byte, &mut model);
    }
    
    let mut output = Vec::with_capacity(4 + enc.output().len() + 8);
    output.extend_from_slice(&(input.len() as u32).to_le_bytes());
    output.extend_from_slice(enc.finish());
    
    output
}

/// Распаковывает данные.
pub fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 {
        return None;
    }

    let orig_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let payload = &data[4..];

    let mut model = FastAdaptiveModel::new();
    let mut dec = FastRangeDecoder::new(payload)?;
    let mut result = Vec::with_capacity(orig_len);

    for _ in 0..orig_len {
        let sym = dec.decode(&mut model);
        result.push(sym);
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_text() {
        let text = b"AXIOMA ultimate universal compressor - zero allocations test payload";
        let comp = compress(text);
        let decomp = decompress(&comp).unwrap();
        assert_eq!(text.to_vec(), decomp);
    }

    #[test]
    fn test_roundtrip_random() {
        let mut data = vec![0u8; 64_000];
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = ((i * 1103515245 + 12345) >> 16) as u8;
        }
        let comp = compress(&data);
        let decomp = decompress(&comp).unwrap();
        assert_eq!(data, decomp, "Ошибка: распакованные данные не совпадают с оригиналом!");
    }
}
