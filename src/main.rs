use std::env;
use std::fs;
use std::process;
use std::time::Instant;
use axioma::{compress, decompress};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Axioma Universal Compressor Core");
        eprintln!("Usage:");
        eprintln!("  {} compress <input_file> <output_file>", args[0]);
        eprintln!("  {} decompress <input_file> <output_file>", args[0]);
        process::exit(1);
    }

    let mode = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];

    match mode.as_str() {
        "compress" => {
            println!("Reading {}...", input_path);
            let input_data = fs::read(input_path).unwrap_or_else(|err| {
                eprintln!("Error reading file: {}", err);
                process::exit(1);
            });

            let start = Instant::now();
            let compressed = compress(&input_data);
            let duration = start.elapsed();

            fs::write(output_path, &compressed).unwrap_or_else(|err| {
                eprintln!("Error writing file: {}", err);
                process::exit(1);
            });

            let ratio = (compressed.len() as f64 / input_data.len() as f64) * 100.0;
            println!("✓ Compressed in {:?}", duration);
            println!("✓ Size: {} -> {} bytes ({:.2}%)", input_data.len(), compressed.len(), ratio);
        }
        "decompress" => {
            println!("Reading {}...", input_path);
            let compressed_data = fs::read(input_path).unwrap_or_else(|err| {
                eprintln!("Error reading file: {}", err);
                process::exit(1);
            });

            let start = Instant::now();
            let decompressed = decompress(&compressed_data).unwrap_or_else(|_| {
                eprintln!("Error: Corrupted file or invalid format");
                process::exit(1);
            });
            let duration = start.elapsed();

            fs::write(output_path, &decompressed).unwrap_or_else(|err| {
                eprintln!("Error writing file: {}", err);
                process::exit(1);
            });

            println!("✓ Decompressed in {:?}", duration);
            println!("✓ Restored size: {} bytes", decompressed.len());
        }
        _ => {
            eprintln!("Unknown mode '{}'. Use 'compress' or 'decompress'.", mode);
            process::exit(1);
        }
    }
}
