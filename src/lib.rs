pub mod entropy;
pub mod codec;

use entropy::FastAdaptiveModel;
use codec::{FastRangeEncoder, FastRangeDecoder};

/// Сжать произвольные байты.
/// Возвращает compressed данные с 4‑байтным LE‑заголовком исходной длины.
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut model = FastAdaptiveModel::new();
    // Гарантируем отсутствие realloc в горячем пути:
    // верхняя оценка размера = вход + 16 байт (flush + запас).
    let mut enc = FastRangeEncoder::with_capacity(input.len() + 16);
    for &byte in input {
        enc.put(byte as usize, &mut model);
    }
    let mut output = Vec::with_capacity(4 + enc.len());
    output.extend_from_slice(&(input.len() as u32).to_le_bytes());
    output.extend_from_slice(&enc.finish());
    output
}

/// Распаковать данные, полученные из `compress`.
pub fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 {
        return None;
    }
    let orig_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let payload = &data[4..];
    let mut model = FastAdaptiveModel::new();
    let mut dec = FastRangeDecoder::new(payload);
    let mut result = Vec::with_capacity(orig_len);
    for _ in 0..orig_len {
        let sym = dec.get(&mut model);
        result.push(sym as u8);
    }
    Some(result)
}
