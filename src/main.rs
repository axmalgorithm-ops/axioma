use axioma;

fn main() {
    let original = b"AXIOMA ultimate universal compressor – zero allocations, ultra speed.";
    let compressed = axioma::compress(original);
    let decompressed = axioma::decompress(&compressed).expect("decompression failed");
    assert_eq!(&decompressed[..], &original[..]);
    println!("✓ Round-trip OK [{} -> {} bytes]", original.len(), compressed.len());

    // Стресс‑тест на 100 KB псевдослучайных данных
    let big: Vec<u8> = (0..100_000).map(|i| (i.wrapping_mul(0x9E3779B9) >> 16) as u8).collect();
    let comp = axioma::compress(&big);
    let decomp = axioma::decompress(&comp).unwrap();
    assert_eq!(decomp, big);
    println!("✓ 100 KB random data round‑trip, ratio = {:.1}%", 
             comp.len() as f64 / big.len() as f64 * 100.0);
}
