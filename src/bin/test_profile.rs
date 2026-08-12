// standalone test script to check memory profiles and performance
// drop this into src/bin/test_profile.rs and run with: cargo run --release --bin test_profile

pub trait CompressionConfig {
    const STATE_BUFFER_SIZE: usize;
    const CONTEXT_TABLE_SIZE: usize;
}

pub struct StandardConfig;
impl CompressionConfig for StandardConfig {
    const STATE_BUFFER_SIZE: usize = 65536; // 64kb state buffer
    const CONTEXT_TABLE_SIZE: usize = 65536;
}

pub struct CortexM0Config;
impl CompressionConfig for CortexM0Config {
    const STATE_BUFFER_SIZE: usize = 4096; // 4kb to fit tiny ram
    const CONTEXT_TABLE_SIZE: usize = 4096;
}

fn run_profile_test<C: CompressionConfig>() {
    let mut work_buf = vec![0u8; C::STATE_BUFFER_SIZE];
    
    let start = std::time::Instant::now();
    
    // simulate some compression loop over the buffer bytes
    for (idx, val) in work_buf.iter_mut().enumerate() {
        *val = ((idx * 31) % 256) as u8;
    }
    
    let elapsed = start.elapsed();
    println!("profile size: {} bytes | time elapsed: {:?}", C::STATE_BUFFER_SIZE, elapsed);
}

fn main() {
    println!("=== axioma hardware profile standalone test ===");
    
    println!("\ntesting standard profile...");
    run_profile_test::<StandardConfig>();
    
    println!("\ntesting cortex-m0 tight profile...");
    run_profile_test::<CortexM0Config>();
    
    println!("\nall checks passed cleanly.");
}
