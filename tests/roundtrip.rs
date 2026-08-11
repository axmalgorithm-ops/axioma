use axioma::{Encoder, Decoder, NoOpPreprocessor, DeltaPreprocessor};

#[test]
fn roundtrip_text() {
    let input = b"Hello, world! This is a test of the axioma range coding compression engine.";
    let mut compressed = Vec::new();
    let mut encoder = Encoder::new(NoOpPreprocessor);

    encoder.compress(input, &mut |data: &[u8]| {
        compressed.extend_from_slice(data);
        Ok(())
    }).unwrap();

    let mut decompressed = vec![0u8; input.len()];
    let mut decoder = Decoder::new(NoOpPreprocessor);
    let n = decoder.decompress(&compressed, &mut decompressed).unwrap();

    assert_eq!(n, input.len());
    assert_eq!(&decompressed[..n], input);
}

#[test]
fn roundtrip_binary() {
    let input: Vec<u8> = (0..=255).collect();
    let mut compressed = Vec::new();
    let mut encoder = Encoder::new(NoOpPreprocessor);

    encoder.compress(&input, &mut |data: &[u8]| {
        compressed.extend_from_slice(data);
        Ok(())
    }).unwrap();

    let mut decompressed = vec![0u8; input.len()];
    let mut decoder = Decoder::new(NoOpPreprocessor);
    let n = decoder.decompress(&compressed, &mut decompressed).unwrap();

    assert_eq!(n, input.len());
    assert_eq!(&decompressed[..n], &input[..]);
}

#[test]
fn roundtrip_delta() {
    let input = vec![10u8, 12, 14, 16, 18, 20, 22, 24];
    let mut compressed = Vec::new();
    let mut encoder = Encoder::new(DeltaPreprocessor::new(1));

    encoder.compress(&input, &mut |data: &[u8]| {
        compressed.extend_from_slice(data);
        Ok(())
    }).unwrap();

    let mut decompressed = vec![0u8; input.len()];
    let mut decoder = Decoder::new(DeltaPreprocessor::new(1));
    let n = decoder.decompress(&compressed, &mut decompressed).unwrap();

    assert_eq!(n, input.len());
    assert_eq!(&decompressed[..n], &input[..]);
}
