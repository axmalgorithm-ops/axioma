use axioma::encoder::Encoder;
use axioma::decoder::Decoder;
use axioma::preprocessor::DeltaPreprocessor;

const CHUNK_SIZE: usize = 4096;

fn main() {
    println!("\n=== AXIOMA REAL-WORLD FIRMWARE BENCHMARK ===\n");
    let datasets = vec![
        ("Telemetry", generate_telemetry(65536)),
        ("Logs", generate_logs(65536)),
        ("Real Firmware", generate_real_firmware(65536)),
        ("High Entropy", generate_high_entropy(65536)),
    ];

    println!("{:<14} | {:<10} | {:<10} | {:<10} | {:<8}", "Dataset", "Orig", "Comp", "Ratio(%)", "Int");
    println!("---------------------------------------------------------------");

    for (name, orig) in datasets {
        let mut compressed_data = Vec::new();
        
        for chunk in orig.chunks(CHUNK_SIZE) {
            let mut temp_buf = Vec::new();
            let mut enc = Encoder::new(DeltaPreprocessor::new());
            let _ = enc.compress(chunk, &mut |d| {
                temp_buf.extend_from_slice(d);
                Ok(())
            });

            if temp_buf.len() < chunk.len() {
                compressed_data.push(1);
                compressed_data.extend_from_slice(&(temp_buf.len() as u16).to_le_bytes());
                compressed_data.extend_from_slice(&temp_buf);
            } else {
                compressed_data.push(0);
                compressed_data.extend_from_slice(chunk);
            }
        }

        let mut decoded_data = Vec::new();
        let mut cursor = 0;
        let mut success = true;
        let mut chunk_idx = 0;
        let original_chunks: Vec<&[u8]> = orig.chunks(CHUNK_SIZE).collect();

        while cursor < compressed_data.len() {
            let flag = compressed_data[cursor];
            cursor += 1;
            let expected_chunk_len = original_chunks[chunk_idx].len();

            if flag == 1 {
                let mut len_bytes = [0u8; 2];
                len_bytes.copy_from_slice(&compressed_data[cursor..cursor+2]);
                cursor += 2;
                let comp_len = u16::from_le_bytes(len_bytes) as usize;
                
                let comp_chunk = &compressed_data[cursor..cursor+comp_len];
                cursor += comp_len;

                let mut dec_buf = vec![0u8; expected_chunk_len];
                let mut dec = Decoder::new(DeltaPreprocessor::new());
                if dec.decompress(comp_chunk, &mut dec_buf).is_err() {
                    success = false;
                    break;
                }
                decoded_data.extend_from_slice(&dec_buf);
            } else {
                let raw_chunk = &compressed_data[cursor..cursor+expected_chunk_len];
                cursor += expected_chunk_len;
                decoded_data.extend_from_slice(raw_chunk);
            }
            chunk_idx += 1;
        }

        let integrity = if success && orig == decoded_data { "OK" } else { "FAIL" };
        let ratio = (compressed_data.len() as f64 / orig.len() as f64) * 100.0;
        println!("{:<14} | {:<10} | {:<10} | {:<10.2} | {:<8}", name, orig.len(), compressed_data.len(), ratio, integrity);
    }
}

fn generate_telemetry(s: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(s);
    let mut val: i16 = 500;
    for i in 0..s/2 {
        val = val.wrapping_add(((i % 3) as i16) - 1);
        v.extend_from_slice(&val.to_le_bytes());
    }
    v
}

fn generate_logs(s: usize) -> Vec<u8> {
    b"[2026-08-11] INFO: Node heartbeat status=active\n".iter().cycle().take(s).copied().collect()
}

// Имитация структуры реальной прошивки: повторяющиеся функции, пустые секции (нули) и векторы
fn generate_real_firmware(s: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(s);
    let function_block = [
        0x00, 0xb5, 0x05, 0x46, 0x14, 0x22, 0x01, 0x21, 
        0xfb, 0xf1, 0xbc, 0xf8, 0x00, 0xbd, 0x00, 0x00
    ];
    while v.len() < s {
        let selector = (v.len() % 5) as u8;
        match selector {
            0 => v.extend_from_slice(&function_block),
            1 => v.extend_from_slice(&[0xFF; 32]), // Padding / Flash memory erase state
            2 => v.extend_from_slice(&[0x00; 64]), // BSS section zeros
            _ => {
                for j in 0..16 {
                    v.push((j as u8).wrapping_add(v.len() as u8));
                }
            }
        }
    }
    v.truncate(s);
    v
}

fn generate_high_entropy(s: usize) -> Vec<u8> {
    let mut state: u64 = 0x517cc1b727220a95;
    (0..s).map(|_| {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        (state.wrapping_mul(0x2545f4914f6cdd1d) >> 32) as u8
    }).collect()
}
