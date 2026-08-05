use axioma::{FastAdaptiveModel, FastRangeDecoder, FastRangeEncoder};
use std::time::Instant;

fn bench_dataset(name: &str, data: &[u8]) {
    println!("=== Benchmark: {} ({:.2} MB) ===", name, data.len() as f64 / 1_048_576.0);

    // 1. Кодирование
    let start_enc = Instant::now();
    let mut encoder = FastRangeEncoder::with_capacity(data.len());
    let mut model_enc = FastAdaptiveModel::new();

    for &byte in data {
        encoder.encode(byte, &mut model_enc);
    }
    let compressed = encoder.finish();
    let enc_duration = start_enc.elapsed();

    let enc_speed = (data.len() as f64 / 1024.0 / 1024.0) / enc_duration.as_secs_f64();
    let ratio = (compressed.len() as f64 / data.len() as f64) * 100.0;

    println!(
        "Encode: {:.2?} | Speed: {:.2} MB/s | Compressed: {} bytes ({:.2}%)",
        enc_duration, enc_speed, compressed.len(), ratio
    );

    // 2. Декодирование
    let start_dec = Instant::now();
    let mut decoder = FastRangeDecoder::new(compressed).expect("Failed to init decoder");
    let mut model_dec = FastAdaptiveModel::new();
    let mut decompressed = Vec::with_capacity(data.len());

    for _ in 0..data.len() {
        let byte = decoder.decode(&mut model_dec);
        decompressed.push(byte);
    }
    let dec_duration = start_dec.elapsed();

    let dec_speed = (data.len() as f64 / 1024.0 / 1024.0) / dec_duration.as_secs_f64();
    let ok = decompressed == data;

    println!(
        "Decode: {:.2?} | Speed: {:.2} MB/s | Integrity: {}",
        dec_duration,
        dec_speed,
        if ok { "PASSED" } else { "FAILED" }
    );
    println!();
}

fn main() {
    println!("--- Axioma Codec Performance Benchmarks ---\n");

    // Тест 1: Повторяющийся текст (2 МБ)
    let text_data = "Axioma fast data compression algorithm based on range coding. "
        .repeat(32_000)
        .into_bytes();
    bench_dataset("Repetitive Text", &text_data);

    // Тест 2: Псевдослучайный поток (1 МБ)
    let mut random_data = vec![0u8; 1_000_000];
    let mut state: u32 = 12345;
    for byte in random_data.iter_mut() {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        *byte = (state >> 24) as u8;
    }
    bench_dataset("Pseudo-random Stream", &random_data);
}
