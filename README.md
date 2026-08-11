```markdown
# axioma

[![no_std](https://img.shields.io/badge/std-no--std-red.svg)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-green.svg)](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html)

A tiny, portable lossless compression library for bytes — no dynamic allocation, no unsafe code, and no external dependencies beyond `core` and `alloc`. It compresses arbitrary data (text, binary, logs, sensor readings) using a modular pipeline: a lightweight preprocessor, an Order‑1 adaptive context model, and a carry‑less binary range coder. The whole thing works in `#![no_std]` environments and on microcontrollers with very little RAM.

---

## Why Axioma?

I wanted a compression tool that:

- runs identically on a 64‑core server, a laptop, and a Cortex‑M0 with 16 KiB of stack,
- makes **zero** heap allocations inside the hot encode/decode loops,
- never panics on corrupted input — every error is a `Result`,
- stays small enough that the entire encoder state fits in 256 KiB (and usually much less),
- and doesn’t care what kind of data you feed it.

Existing compressors tend to specialise: one is good for JSON, another for sensor dumps, a third for text. Axioma doesn’t try to be the best at any single category; instead, it’s a decent generalist that you can drop into a firmware binary without pulling in a C toolchain or a dynamic allocator.

The engine itself is just a binary range coder. The clever bits live in the **preprocessor** — a swappable filter that transforms the input into something the coder can predict more easily. There’s a delta filter for numeric sequences, a text normaliser, and a few others (more planned). The context model is Order‑1 by default, so it picks up local byte correlations without eating all your memory.

---

## Built on a Smartphone

This project was written, tested, and compiled 100 % on a Poco F8 Pro using Termux. No laptop, no external keyboard, no IDE — just a 6‑inch screen, `nano`, and the Rust compiler.

I’m not going to pretend it was a pleasant development experience. Scrolling through borrow‑checker errors that stretch longer than the screen is tall becomes an exercise in patience. Typing `pub struct RangeEncoder<'a, W> where W: FnMut(&[u8]) -> Result<(), Error>` with thumbs on a glass keyboard is as tedious as it sounds. There’s no “go to definition”, no inline type hints — just `cargo check` and squinting at line numbers.

But there’s an upside. When the compiler is your only feedback loop, you learn to think carefully before you write. You keep things small, because a 400‑line file is already a pain to navigate. You avoid abstractions that might introduce unexpected generic parameters, because unravelling them on a phone screen is a special kind of torture. The result is a codebase that’s genuinely simple — not because I’m a minimalist, but because I was forced to be one.

Also, `cargo test` on a phone is surprisingly fast, and Termux gives you a real Linux environment. If I can build this on a phone, you can definitely build it on whatever hardware you have.

---

## Architecture Overview

Axioma treats compression as a pipeline:

```

raw bytes → [preprocessor] → [context model] → [range coder] → compressed bytes

```

- **Preprocessor** (`trait Preprocessor`): transforms the input to reduce entropy before modeling. The included `DeltaPreprocessor` handles fixed‑width integer sequences (1/2/4/8 bytes) by emitting XOR differences. A `NoOpPreprocessor` passes data straight through. Filters are statically bounded — they use only a few bytes of internal state and never allocate.
- **Context model** (`ContextModel`): an Order‑1 adaptive probability tree with 256 binary trees (one per previous byte). Each tree has 255 probability nodes, each stored as a single byte updated by a simple finite‑state machine. The model is heap‑allocated once at construction (via `Vec::with_capacity` → `Box<[BitTree; 256]>`) so that the stack stays tiny — critical for microcontrollers. After that, the hot path only reads and writes the already‑allocated structure.
- **Range coder** (`RangeEncoder` / `RangeDecoder`): a 32‑bit carry‑less binary range coder that handles carry propagation correctly (the tricky 0xFF accumulation). It calls an output closure every time bytes need to be flushed, which makes it completely independent of any particular I/O mechanism — just pass a lambda that writes to a buffer, a serial port, or a `std::io::Write`.

The whole encoder state (preprocessor + model + coder registers) is under 70 KiB in the default configuration, well within the 256 KiB budget.

---

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
axioma = { git = "https://github.com/axmalgorithm-ops/axioma" }
```

Compress and decompress in memory:

```rust
use axioma::{Encoder, Decoder, NoOpPreprocessor};

let original = b"Hello, world! This is a test of the axioma engine.";

// Compress
let mut encoder = Encoder::new(NoOpPreprocessor);
let mut compressed = Vec::new();
encoder
    .compress(original, &mut |chunk| {
        compressed.extend_from_slice(chunk);
        Ok(())
    })
    .unwrap();

// Decompress
let mut decoder = Decoder::new(NoOpPreprocessor);
let mut decompressed = vec![0u8; original.len()];
let written = decoder
    .decompress(&compressed, &mut decompressed)
    .unwrap();

assert_eq!(&decompressed[..written], &original[..]);
```

For numeric sequences, use DeltaPreprocessor::new(width) instead of NoOpPreprocessor — it usually improves compression significantly.

The API is fully #![no_std] compatible; the only reason you might need alloc is to build the ContextModel once at startup. The hot loops never touch the heap.

---

License

Dual‑licensed under MIT or Apache‑2.0, at your option.

---

If you find bugs or want to improve the preprocessors, PRs are welcome. I can’t promise I’ll review them on my phone, but I’ll try.

```
