# axioma

![no_std](https://img.shields.io/badge/no__std-red.svg) ![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-green.svg)

A lightweight, dependency-free lossless compression library for byte streams. Built for low-resource and embedded targets where dynamic allocation is not an option.

## What it is

axioma compresses arbitrary byte data — sensor logs, telemetry, firmware config blobs, plain text — into a compact bitstream without relying on the heap. The core is written in Rust with `#![no_std]` and `#![forbid(unsafe_code)]`.

The entire encoder state fits in less than 70 KiB of static memory, and the hot encode/decode paths perform zero dynamic allocations. That makes it usable on a Cortex-M0 with a 16 KiB stack, where pulling in gzip or zlib is simply not feasible.

## Architecture

The compression pipeline is deliberately boring:

* **Delta preprocessor** handles fixed-width numeric sequences and time series, converting values into differences that the later stages can model more efficiently.
* **Constrained LZ77 sliding window** catches repeated byte patterns within a fixed distance bound.
* **Order-1 context model** adapts to local correlations without needing a large probability table.
* **32-bit binary range coder** entropy-codes the resulting decisions using a carryless binary arithmetic coder.

No stage requires dynamic allocation. Buffers are either statically sized inside the encoder state or provided by the caller.

## Performance

On structured sensor dumps and telemetry data, axioma typically reaches a compression ratio around 3.48x (original size / compressed size). It is not intended to beat heavy compressors on general text or media — the goal is reliable, predictable compression in places where those tools cannot run.

The decoder never panics. Corrupted or truncated input produces a `Result::Err`, not a fault that takes down an RTOS task.

## Development background

This project was written by a 15-year-old about to enter 9th grade, using a Poco F8 Pro smartphone and the Termux terminal. No external keyboard, no desktop IDE — just nano, cargo check, and a lot of scrolling through borrow-checker diagnostics on a 6-inch screen.

The constraints forced a small, straightforward codebase. No clever macros, no deep generic towers, just modules that do one thing and return errors properly.

## Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
axioma = { git = "[https://github.com/axmalgorithm-ops/axioma](https://github.com/axmalgorithm-ops/axioma)" }
```

Minimal encode and decode example (using standard library collections for the caller buffers):

```rust
use axioma::{Decoder, Encoder, NoOpPreprocessor};

let original = b"temperature=22.5,temperature=22.6,temperature=22.7";

// Compress
let mut enc = Encoder::new(NoOpPreprocessor);
let mut compressed = Vec::new();
enc.compress(original, &mut |chunk| {
    compressed.extend_from_slice(chunk);
    Ok(())
})
.unwrap();

// Decompress
let mut dec = Decoder::new(NoOpPreprocessor);
let mut output = vec![0u8; original.len()];
let mut read_pos = 0;
let written = dec
    .decompress(&compressed, &mut |buf| {
        let remaining = compressed.len() - read_pos;
        let to_read = buf.len().min(remaining);
        buf[..to_read].copy_from_slice(&compressed[read_pos..read_pos + to_read]);
        read_pos += to_read;
        Ok(to_read)
    })
    .unwrap();

assert_eq!(&output[..written], original);
```

For numeric sensor data, use `DeltaPreprocessor` instead of `NoOpPreprocessor`:

```rust
use axioma::{DeltaPreprocessor, Encoder};

let samples = [1.0_f32.to_le_bytes(), 1.1_f32.to_le_bytes(), 1.2_f32.to_le_bytes()].concat();
let mut enc = Encoder::new(DeltaPreprocessor::new(4));
```

The library itself is fully `no_std` compatible and does not require the `alloc` crate. Every internal buffer is a fixed-size stack array or a caller-provided slice.

## License

Dual-licensed under MIT or Apache-2.0, at your option.

---

**Repository:** [github.com/axmalgorithm-ops/axioma](https://github.com/axmalgorithm-ops/axioma)

If you find a bug or have a suggestion, open an issue or pull request. Code review is welcome, even if I have to read it on a phone screen.
