# axioma

A lightweight general-purpose compression thing built on fast range coding and a tiny adaptive entropy model.
No dictionaries. No heavy deps. Just math and a few buffers.

## What's inside

- **Fast range coding** – encodes symbols using integer arithmetic, divisions kept to a minimum.
- **Fast adaptive model** – updates symbol probabilities on the fly, no static tables.
- **Streaming-friendly** – feed data in chunks, get compressed output out.
- **Portable** – runs fine on ARM/Mobile, no exotic CPU features needed.
- **Tested on real data** – see benchmarks below.

## Quick start

```bash
# clone and test
git clone https://github.com/axmalgorithm-ops/axioma
cd axioma
cargo test

## License

`axioma` is dual-licensed:
- **Open Source:** Licensed under [GNU General Public License v3.0](LICENSE) (GPLv3).
- **Commercial:** For closed-source, proprietary, or commercial distribution without GPLv3 copyleft restrictions, contact the repository owner.

## Benchmarks & Performance

Tested on **ARM64 (Qualcomm Snapdragon / Poco F8 Pro)** in a local mobile environment:

| Metric | axioma | Note |
| :--- | :--- | :--- |
| **Compression Throughput** | ~5.2 MB/s | Pure integer arithmetic, zero SIMD |
| **Decompression Throughput** | ~14.8 MB/s | Single-thread streaming decode |
| **Memory Overhead (RAM)** | < 256 KB | Fixed-size state, zero dynamic heap allocations |
| **External Dependencies** | 0 | Pure standard library Rust |

### Design Focus & Methodology
- **Hardware Profile:** Snapdragon (ARM64 / Android).
- **Architecture Trade-offs:** Unlike dictionary-based algorithms (such as LZ4 or zstd) that require multi-megabyte window buffers for high throughput, `axioma` is architected for strict streaming entropy coding with minimal RAM usage and zero division operations in hot loops.
- **Reproducibility:** Benchmarked using release builds (`cargo build --release`). SIMD vectorization and hardware-specific intrinsics are planned for upcoming releases.
