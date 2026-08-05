```markdown
# axioma

**A zero-alloc, shift‑only, purely model‑driven streaming lossless compression engine for Rust.**

axioma doesn't rely on static dictionaries, hardcoded tables, or heap allocations on the hot path. Every coding decision is driven by a live, adaptive probability model that updates on the fly — no divisions, no lookups, just math and shifts.

---

## Features

- **Zero heap activity on the hot path**  
  All state lives on the stack. No `Box`, no `Vec`, no allocator traffic where it counts.

- **Division‑free arithmetic**  
  All arithmetic that touches the probability model is implemented with shifts only. No `div`, no `idiv`, no pipeline stalls from integer division.

- **No hardcoded tables**  
  There are no static dictionaries, no pre‑computed Huffman trees, no frozen context tables. Everything is inferred at runtime from the model.

- **Real‑time adaptive probability stream**  
  Probabilities are updated continuously as bytes flow through the encoder/decoder. The model adapts to the data, not the other way around.

- **Truly streaming**  
  Encode and decode incrementally. axioma does not need the entire payload in memory.

- **`no_std` friendly**  
  No global allocator required. Works on bare‑metal targets provided you can supply a few stack‑resident buffers.

---

## Quick Start

Add axioma to your project:

```bash
cargo add axioma
```

Or in Cargo.toml:

```toml
[dependencies]
axioma = "0.1.0"
```

---

Usage

```rust
use axioma::{Encoder, Decoder};

// Some data to compress
let original = b"axioma: shift-only, zero-alloc, model-driven compression";

// Encode
let mut encoder = Encoder::new();
let mut compressed = Vec::new();
encoder.encode(original, &mut compressed).expect("encode failed");
encoder.finish(&mut compressed).expect("flush failed");

// Decode
let mut decoder = Decoder::new();
let mut recovered = vec![];
decoder.decode(&compressed, &mut recovered).expect("decode failed");
decoder.finish(&mut recovered).expect("flush failed");

assert_eq!(original, &recovered[..]);
```

Encoder and Decoder are symmetric, streaming, and never touch the heap internally.

---

How It Works

axioma treats compression as a prediction problem.

Internally, it maintains a lightweight probability distribution over the input alphabet. For each symbol, the model:

1. Predicts the next symbol using the current distribution.
2. Encodes (or decodes) it using a shift‑only arithmetic coder.
3. Updates the distribution immediately — no batches, no blocks, no delay.

There are no static tables anywhere. Even the initial model state is mathematically derived, not stored.

Because the entire hot path uses only register‑to‑register operations and shift arithmetic, axioma avoids many of the micro‑architectural penalties that plague traditional compressors (division latency, branch mispredictions on table lookups, cache misses from static dictionaries).

The result is a compressor that is surprisingly fast for something that carries zero prior assumptions about the data it’s going to see.

---

Why “axioma”?

The name comes from axiom — a starting assumption from which everything else is derived. axioma starts with almost nothing: a minimal mathematical prior and a set of update rules. Everything else emerges from the data stream.

---

License

MIT

---

Pull requests, issues, and brutal technical critique welcome.
