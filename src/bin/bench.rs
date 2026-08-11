extern crate alloc;

use axioma::preprocessor::Preprocessor;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::ToString;

fn main() {
    println!("\n=== AXIOMA DEEPSEEK LZ77 FINAL BENCHMARK ===\n");
    let datasets = vec![
        ("Repeating Logs", generate_repeating_logs(16384)),
        ("Telemetry", generate_telemetry(16384)),
    ];

    println!("{:<16} | {:<10} | {:<10} | {:<10} | {:<8}", "Dataset", "Orig", "Comp", "Ratio(%)", "Int");
    println!("------------------------------------------------------------------");

    for (name, orig) in datasets {
        // Encoder
        let mut enc_prep = axioma::preprocessor::Lz77Preprocessor::new();
        let mut comp_buf = vec![0u8; orig.len() * 3 + 1024];
        
        let (_, mut comp_len) = enc_prep.process(&orig, &mut comp_buf);
        let mut flush_buf = vec![0u8; 4096];
        let flushed = enc_prep.flush(&mut flush_buf);
        
        if comp_len + flushed <= comp_buf.len() {
            comp_buf[comp_len..comp_len + flushed].copy_from_slice(&flush_buf[..flushed]);
            comp_len += flushed;
        }
        comp_buf.truncate(comp_len);

        // Decoder (используем reverse_process и reverse_flush)
        let mut dec_prep = axioma::preprocessor::Lz77Preprocessor::new();
        let mut dec_buf = vec![0u8; orig.len() * 3 + 1024];
        
        let (_, mut dec_len) = dec_prep.reverse_process(&comp_buf, &mut dec_buf);
        let flushed_dec = dec_prep.reverse_flush(&mut flush_buf);
        
        if dec_len + flushed_dec <= dec_buf.len() {
            dec_buf[dec_len..dec_len + flushed_dec].copy_from_slice(&flush_buf[..flushed_dec]);
            dec_len += flushed_dec;
        }
        dec_buf.truncate(dec_len);

        let is_ok = orig == dec_buf;
        let integrity = if is_ok { "OK".to_string() } else { "FAIL".to_string() };
        let ratio = (comp_buf.len() as f64 / orig.len() as f64) * 100.0;
        println!("{:<16} | {:<10} | {:<10} | {:<10.2} | {:<8}", name, orig.len(), comp_buf.len(), ratio, integrity);
    }
}

fn generate_repeating_logs(s: usize) -> Vec<u8> {
    let phrase = b"ERROR 503: Service Unavailable at /api/v1/telemetry -- retrying...\n";
    phrase.iter().cycle().take(s).copied().collect()
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
