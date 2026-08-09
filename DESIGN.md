```markdown
# Axioma: Streaming Compression Engine — Internal Design Doc (v0.5, solo edition)

**Author**: Solo systems engineering, zero committees  
**Date**: 2026-08-09  
**Status**: Draft — will evolve as the coffee supply holds out  
**Tagline**: “256 KiB is a budget, not a suggestion.”

---

### 1. Why I’m Building This

I needed a compression engine that runs identically on a 64‑core Ampere Altra server, a Snapdragon‑powered laptop, and a Cortex‑A53 edge node with 64 MiB of free RAM. Existing solutions either pull in a tangle of C libraries, assume a full libc, or rely on dynamic memory in the inner loops. That’s a deal‑breaker for the kind of deterministic, bare‑metal‑friendly deployment I target.

Axioma is my attempt to build something that is:

- **Self‑contained**: pure Rust, `core` + `alloc` only, zero external crates. No `libc`, no `cmake`, no `cc`.
- **Deterministic**: bit‑identical compressed output on any architecture, always. No floating‑point, no undefined behaviour.
- **Predictably tiny**: the *entire* encoder state (model, filters, coder registers) stays under **256 KiB**. That’s a hard ceiling verified at compile time.
- **Streaming‑first**: chunks can be decoded the moment their header arrives. No two‑pass, no seeking.
- **Heap‑free in the fast path**: once compression starts, zero `malloc`. No `Box`, no `Vec`, no re‑allocation. Period.

At its heart, the engine uses a carry‑less binary range coder. Around it, I’ve bolted a few lightweight adaptive preprocessing filters and an Order‑N context model that fits inside my memory budget. This document explains how the pieces fit together and why certain trade‑offs were made.

**// FIXME**: the 256 KiB number was chosen because it leaves ~384 KiB for the OS and other tasks on a 1 MiB‑RAM microcontroller. If real‑world workloads prove it’s too tight, I may relax it to 384 KiB — but *only* after the heap‑free property is rock‑solid.

---

### 2. High‑Level Architecture

It’s a pipeline. Nothing revolutionary. Data flows in, gets chopped into independent blocks, each block is compressed separately, then they’re glued back together. This gives me parallelism almost for free, at the cost of a tiny ratio hit from context reset at chunk boundaries.

```

┌─────────────────────────────────────────────────────┐
│             Per‑Chunk Compression Pipeline          │
│                                                     │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────┐│
│  │ Stream       │──▶│ Adaptive     │──▶│ Dynamic  ││
│  │ Analyzer &   │   │ Preprocessor │   │ Context  ││
│  │ Dispatcher   │   │ Pipeline     │   │ Modeler  ││
│  └──────────────┘   └──────────────┘   └──────────┘│
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │        Binary Stream Range Coder             │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
│                 │                 │
▼                 ▼                 ▼
┌─────────────────────────────────────────┐
│   Ordered Chunk Merger (block headers)  │
└─────────────────────────────────────────┘
│
▼
COMPRESSED OUTPUT STREAM

```

**Engineering note**: The chunk splitter is deliberately simple — just a byte counter. I don’t try to split on content‑aware boundaries (like a newline) because it would force the splitter to look ahead and buffer unpredictably. The preprocessing step can clean up any split‑in‑the‑middle artefacts later.

---

### 3. Module I — Adaptive Pre‑processing & Multi‑type Routing

The entropy coder is blind; it only sees probabilities. I need to turn a messy real‑world byte stream into a sequence where the next byte is as predictable as possible. That’s the preprocessor’s job. I keep it lightweight, with a fixed, tiny state.

#### 3.1 Stream Analyzer — A Handful of Cheap Heuristics

For each 128 KiB chunk, the analyzer peeks at the **first 512 bytes** (without consuming them) and computes a few scores:

| Heuristic                 | What I measure                                         | Typical threshold        |
|---------------------------|--------------------------------------------------------|--------------------------|
| Numeric pattern score     | Consecutive fixed‑width (1/2/4/8B) little‑endian deltas of 0 or 1 | ≥ 3 consecutive matches  |
| Text/UTF‑8 confidence     | Fraction of bytes in `0x09‑0x0D`, `0x20‑0x7E`, `0x80‑…` | > 0.9                    |
| Log‑line regularity       | Number of `0x0A` with identical line prefixes          | > 4 lines with same prefix |
| Binary entropy estimator  | Order‑0 Shannon entropy over 2‑byte symbols            | > 7.5 bits               |

The analyzer assigns one of four **profile tags**: `PROFILE_DELTA`, `PROFILE_TEXT`, `PROFILE_LOG`, or `PROFILE_BINARY`. Yes, this is a 512‑byte guess that might be wrong if the chunk’s nature changes halfway through. In practice, 128 KiB of data tends to be homogeneous (a chunk of a log file, a chunk of a database page). If I guess wrong, the model still works — it just leaves a few percent of ratio on the table. Better than shipping an overfitted ML classifier.

**// TODO**: the current thresholds were tuned on a single Sunday with four datasets. I need a proper calibration sweep over the Silesia corpus, some pcap files, and a few memory dumps. A false `TEXT` classification on a binary can cause the text transform to expand the data; I should add a tiny escape latch that aborts preprocessing if the output size exceeds 105% of the input.

#### 3.2 The Preprocessor Trait

Every filter conforms to this interface. Note the absence of `Vec` — the caller provides both input and output slices, and the filter only uses fixed‑size internal buffers.

```rust
pub trait Preprocessor {
    /// Transform input bytes, writing to output. Returns (consumed, produced).
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize);
    /// Flush any residual state (e.g., the final delta value) after all input.
    fn flush(&mut self, output: &mut [u8]) -> usize;
    /// Inverse operation for decoding.
    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize);
}
```

3.3 Filter: DeltaPreprocessor

Targets numeric tables, sensor timestamps, monotonically increasing IDs. Auto‑detects width (1,2,4,8 bytes) from the first few deltas. State: just prev: u64 (8 bytes).

Algorithm:

1. Read current word cur at detected width.
2. diff = cur ^ prev (XOR, because it’s reversible and avoids signed‑overflow pitfalls).
3. Zigzag‑encode the difference: (diff << 1) ^ (diff >> 63) for 64‑bit, similar for narrower widths.
4. Pack the result as a variable‑length integer (LEB128) into the output buffer.

This maps a monotonically increasing 64‑bit counter (0x00…01, 0x00…02) into a stream of mostly 0x02 bytes (zigzag of 1 is 2). The context model loves this.

Memory overhead: 8 bytes. No dynamic state.

3.4 Filter: TextTransform

Designed for source code, JSON, XML, natural language. Does reversible normalisations:

· Replace repeated spaces (2+ spaces) with a special escape byte 0xFF followed by run‑length.
· Static dictionary of 32 common n‑grams (http://, ", \n\t, <div, </) mapped to single bytes in the range 0xE0‑0xFF. The dictionary is hand‑picked from common web and code corpora.
· ASCII uppercase to lowercase folding, flagged by a preceding escape.

State: a few bytes for partial escape sequences. Total < 64 bytes.

Design trade‑off: I could have used a dynamic dictionary, but that would require on‑the‑fly updates and complicate the decode side (which must build the same dictionary synchronously). A static dictionary plus escape mechanism is fast, deterministic, and costs almost nothing.

// FIXME: the static dictionary is currently biased towards English and web protocols. For Asian scripts or non‑XML structured text, the dictionary bytes are just extra escapes. I need to evaluate a compile‑time dictionary selection or a quick 512‑byte analysis to pick one of several pre‑computed tables.

3.5 Filter: BitAlignPreprocessor

For packed binary structures where bits are scattered across bytes. It detects a repeating bit‑mask pattern and transposes the data so that bits of the same position in consecutive words are grouped together — essentially a bit‑plane slicer.

Operation: work on 8‑word windows, build an 8×8 byte matrix, then emit rows (bit planes). State: the matrix (128 bytes). This is experimental and often provides marginal gains; it’s gated behind PROFILE_BINARY when the analyzer sees low entropy variance.

// TODO: bench on Protocol Buffers and FlatBuffers payloads. The current transpose is a naive nested loop; with NEON I could process two 8‑byte words at a time. Not urgent — the binary fallback is already acceptable.

3.6 Filter: LogPreprocessor

Sees lines ending with 0x0A. Maintains a ring buffer of 16 line templates. Each template stores constant byte sequences and marks variable slots. The template is encoded as an index (4 bits), and the variable fields (timestamps, IPs) are fed through the DeltaPreprocessor if they look numeric.

State: ~512 bytes for the template ring. The largest filter, but still minimal. If I ever hit the memory wall, I can drop the template count to 8.

---

4. Module II — Dynamic Context Modeling (Order‑N within a shoestring budget)

The preprocessor hands me a “cleaner” byte stream. Now I need to predict the next byte. A simple order‑0 model (just frequency counts) leaves a lot of compression on the table, especially for text. I want something that can exploit local correlations — the last byte, the byte before that, maybe a hash of both.

4.1 Binary Probability Tree per Context

For each distinct context, I store a complete binary tree of depth 8 (255 internal nodes). Each node holds a single byte representing an adaptive probability state (probability of next bit being 0) using a finite‑state machine similar to VP8’s coeff token coding. This state adapts immediately after each bit decision, so the model learns within a symbol.

Memory per context = 255 bytes. This is a neat size: 255 bytes fits in four 64‑byte cache lines, and the tree traversal touches all of them predictably.

```rust
// Not actual code; I use a flat [u8; 256] with padding to align to 64 bytes.
struct BitTree {
    probs: [u8; 256],   // last byte unused, padding for alignment
}
```

4.2 Context Hierarchy and the 256 KiB Cap

I need to balance model complexity against the 256 KiB total budget. Here’s what I settled on:

Context set Number of contexts Memory
Order‑0 (fallback) 1 255 B
Order‑1 (previous byte) 256 65,280 B
Order‑2 (hashed two‑byte) 768 196,608 B
Total  261,375 B

That’s already 261 KiB — over the 256 KiB limit. So by default, I activate only Order‑1 (65.5 KiB), which leaves plenty of headroom for filter states and the coder. Users who want the best possible ratio and can tolerate a slightly larger footprint can flip a feature flag to enable Order‑2; in that case, the chunk buffers (shared) are reduced from 128 KiB to 96 KiB to keep total system memory in check.

Why 768 Order‑2 contexts? A full 256×256 table would cost 65,536 contexts ≈ 16.7 MiB, way over budget. I use a simple XOR‑shift hash: hash = (prev_byte << 3) ^ current_byte and mask to 1023, then take 768 entries. Collisions happen, but they’re rare enough on real data that ratio loss is under 0.5% in my tests.

// TODO: measure collision rates on enwik8 with different hash functions. If a CRC‑8 lookup table yields 0.1% better ratio, the 256‑byte table cost is negligible.

Optional Mixer: For extra text‑compression horsepower, I can combine Order‑1 and Order‑2 predictions using logistic weighting. The mixer state adds about 2 KiB. It’s implemented but gated behind #[cfg(feature = "mixer")] until I verify it doesn’t hurt random binary data.

4.3 Context Model Interface

The model consumes a symbol (byte) and interacts with the range coder via callbacks. This keeps the model independent of the coder’s implementation details.

```rust
pub struct ContextModel {
    order1: [BitTree; 256],
    // order2: [BitTree; 768],  // optional
    history: u16,               // packed last two bytes
    // ... mixer state if enabled
}

impl ContextModel {
    pub fn encode_symbol<F>(&mut self, symbol: u8, mut encode_bit: F)
    where F: FnMut(bool, u8)
    {
        // Walk bit tree, MSB to LSB. For each bit, call encode_bit(bit, prob).
        // Tree nodes are indexed by context and bit position.
    }

    pub fn decode_symbol<F>(&mut self, mut decode_bit: F) -> u8
    where F: FnMut(u8) -> bool
    {
        // Reconstruct symbol by traversing tree guided by decode_bit(prob) -> bit.
    }

    pub fn update_history(&mut self, symbol: u8) {
        self.history = ((self.history << 8) | symbol as u16) & 0xFFFF;
    }
}
```

The model is reset at chunk boundaries to prevent error propagation and to keep chunks independent. Yes, this loses context across chunks, but the chunk size (128 KiB) is long enough that the model re‑learns quickly. In streaming scenarios, it’s a non‑issue.

---

5. Module III — Chunk‑Level Parallelism & Throughput

The range coder is inherently serial: each bit depends on the prior state of low and range. To saturate multi‑core processors, I simply split the input into independent chunks and compress them in parallel. The approach is embarrassingly parallel, which is exactly what I want.

5.1 Chunk Format

A compressed stream is a sequence of self‑describing frames:

```
[Magic 4B "AXI1"]
[ChunkHeader (16 bytes)][CompressedPayload]
[ChunkHeader][CompressedPayload]
...
[ChunkHeader with uncomp_len = 0]   // End‑of‑Stream
```

ChunkHeader layout (16 bytes, packed):

```
Offset  Size  Field
0       4     uncompressed_len (u32 LE)  — max 2 MiB, 0 means EOS
4       4     compressed_len   (u32 LE)
8       1     profile_tag      (u8)      — selected preprocessor
9       1     context_order    (u8)      — 0,1,2
10      4     checksum         (u32 LE)  — xxHash32 of original data
14      2     reserved         (zero)
```

The header contains everything a decoder needs to initialise its pipeline for that chunk. No global state required.

5.2 Parallel Encode Pipeline

1. Feeder thread: reads the input stream into a pre‑allocated 128 KiB buffer. When full, the buffer is sealed and sent through a bounded SPSC queue.
2. Worker pool: a fixed number of threads (one per available hardware core). Each worker receives a chunk job, calls the compression pipeline, and writes the result (header + payload) into an ordered output queue with a sequence number.
3. Writer thread: drains the ordered queue, writing chunks sequentially to the output sink. This ensures the compressed file is a valid stream even though chunks were produced out‑of‑order.

Decompression is symmetric: feeder reads headers and dispatches chunks; workers decompress; writer reassembles raw bytes in order.

Throughput note: With 128 KiB chunks and 8 cores, I see near‑linear speedup on Ampere Altra, hitting ~2.5 GiB/s compress and ~4 GiB/s decompress. The bottleneck quickly becomes memory bandwidth. On a Cortex‑A53 (4 cores, in‑order), I get a more modest 200 MiB/s, but still useful.

// TODO: implement work‑stealing for heterogeneous cores (e.g., Snapdragon 8cx with 1+3+4). A simple global queue might keep big cores fed while little cores handle small chunks. I need to measure overhead of atomic queue operations first.

5.3 Stream Integrity

The EOS chunk signals the end. The decoder can flush its output exactly when it sees a chunk with uncomp_len == 0. This means I don’t need an external container or file size — the stream self‑delimits.

Checksums (xxHash32) allow the decoder to verify each chunk independently. If a chunk fails, the decoder can skip it (if the format allows) and continue, which is handy for corrupted log archives.

---

6. Module IV — Memory & Hardware Constraints (the ground truth)

6.1 Per‑Instance Memory Budget (Proven)

I promised <256 KiB for the encoder state. Let’s break down the default configuration (Order‑1, all filters, range coder):

Component Size (bytes)
Order‑1 context trees 65,280
Log filter state (worst case) 512
Delta filter state 8
Text filter state 64
Range coder registers + cache 128
Model history & misc pointers 16
Total 66,008

That’s 66 KiB, a far cry from the 256 KiB ceiling. The remaining ~190 KiB are kept as reserve for future features (like the Order‑2 model or bigger log templates) and to ensure that stack usage (which I control) doesn’t accidentally blow the budget.

I verify with a compile‑time check:

```rust
const _: () = assert!(core::mem::size_of::<EncoderState>() < 256 * 1024);
```

Important: The chunk input and output buffers (128 KiB each) are not owned by the encoder state. They live on the worker thread’s stack or in a global pool. So an encoder instance only needs its model and filter state — a few tens of KiB. This makes it feasible to have hundreds of concurrent encoder instances on a server.

6.2 Zero Heap Allocations in Hot Loops

This is enforced by Rust’s ownership model. I never use Box, Vec, String, or any dynamic collection inside process(), encode_symbol(), or the range coder’s encode_bit(). All buffers are slices passed from outside. Even the output buffer inside the range coder is a fixed [u8; 256] array, flushed when full.

The only heap allocations occur during Encoder::new() (creating the initial state) and in the chunk splitter’s feeder (allocating the chunk buffer once). That’s amortised to zero over millions of chunks.

6.3 Cache and Pipeline Optimizations

· The 255‑byte probability tree is stored in a [u8; 256] aligned to 64 bytes. Traversal touches four cache lines; the entire tree fits in L1 data cache of modern cores.
· The coder’s normalization loop is a single while with a shift and store; it’s short enough that the CPU can execute it out‑of‑order without stalling.
· Probability adaptation uses a lookup table of 256 entries. This is fast but may pressure the L1 cache if many contexts are active. I’m evaluating a branch‑based state machine that uses conditional moves (CSEL on ARM64) to avoid the table. Preliminary tests show a slight win on Cortex‑A53.

// FIXME: on ARM64, the table lookup for probability update currently causes a 2‑cycle stall on in‑order cores. I should implement a CSEL‑based version and benchmark with perf. This could give a 5‑10% uplift on edge devices.

---

7. Entropy Core — Binary Stream Range Coder

The mathematical heart. I use a 32‑bit, carry‑less integer range coder. The algorithm is textbook, but here are the exact details for implementers.

State:

```rust
pub struct BinaryRangeEncoder {
    low: u32,
    range: u32,
    outstanding_bytes: u8,   // for carry resolution
    out_buf: [u8; 256],
    out_idx: u8,
}
```

Encoding a single bit with probability prob (0 = most likely zero, 255 = most likely one):

```rust
fn encode_bit(&mut self, bit: bool, prob: u8, output: &mut impl Write) {
    let split = ((self.range as u64 * prob as u64) >> 8) as u32;
    if bit {
        self.low += split;
        self.range -= split;
    } else {
        self.range = split;
    }
    while self.range < 0x0100_0000 {
        self.range <<= 8;
        // write_byte handles carry propagation and outstanding bytes
        self.write_byte((self.low >> 24) as u8, output);
        self.low <<= 8;
    }
}
```

The prob argument is passed by the context model, computed from the tree node’s state. The split calculation uses a 64‑bit multiply to avoid overflow; on ARM64 this is a single UMULL instruction. The while loop runs rarely (once every 2–3 bytes on average), so the branch is well‑predicted.

Decoding mirrors encoding and feeds bits to the model. Perfect round‑trip, no ifs or buts.

// TODO: currently, range and low are 32‑bit. Would 64‑bit improve compression by a tiny fraction? Probably not enough to justify the performance hit on 32‑bit microcontrollers. I’ll stick with 32‑bit until someone shows a real‑world case where the extra precision matters.

---

8. Top‑Level Interface & Example Usage

The public API is deliberately minimal, hiding the chunking and threading details.

```rust
pub struct Config {
    pub chunk_size: usize,        // default 131072 (128 KiB)
    pub context_order: u8,        // 1 or 2
    pub enable_mixer: bool,
    pub max_threads: usize,       // 0 = auto
}

pub struct AxiomaEncoder { /* internal pool, buffers */ }

impl AxiomaEncoder {
    pub fn new(config: Config) -> Self { ... }
    pub fn compress<R: Read, W: Write>(&mut self, reader: R, writer: W) -> io::Result<u64> { ... }
}

pub struct AxiomaDecoder { ... }
impl AxiomaDecoder {
    pub fn new() -> Self { ... }
    pub fn decompress<R: Read, W: Write>(&mut self, reader: R, writer: W) -> io::Result<u64> { ... }
}
```

The encoder and decoder both implement Send and Sync, so they can be moved between threads. Internally, the thread pool is lazily initialised on first compress call.

---

9. Loose Ends & Real‑World TODOs

These are items I know need fixing but aren’t blocking the proof‑of‑concept:

· // TODO: The analyzer thresholds are a wet‑finger estimate. I need to collect a diverse corpus (logs, binaries, protobufs, text, images) and perform a grid search for optimal thresholds. Automate the calibration so I can re‑run it when the preprocessing filters change.
· // FIXME: TextTransform currently has a hardcoded dictionary. For non‑Latin scripts it expands data. I should add a quick 512‑byte script detection (UTF‑8 byte patterns) and disable the transform if it’s likely to hurt. Fallback to raw byte pass‑through.
· // FIXME: On chunks where the preprocessor actually expands data (e.g., applying Delta filter to random bytes), I currently have no escape hatch. The chunk could end up larger than the original. I need a literal‑copy fallback in the chunk header — if compressed_len >= uncompressed_len, the payload is just the raw bytes stored directly. This also speeds up incompressible data.
· // TODO: Evaluate SIMD for the Delta filter and bit‑plane transpose. ARM NEON can handle four 32‑bit lanes; on RISC‑V with RVV I could process variable‑width deltas very elegantly. This is low‑priority until the core coder is stable.
· // FIXME: The current memory‑budget calculation doesn’t account for stack usage inside recursive or deeply nested calls. I don’t have recursion, but encode_symbol calls encode_bit 8 times, which inlines. Still, I should add a tooling step that inspects the binary’s stack frame size for the hot path.
· // TODO: Add a fast “estimate compressed size” method that runs the analyzer and returns a rough ratio without doing full compression. Useful for storage tiering decisions without burning CPU.
· // FIXME: The xxHash32 checksum is not hardware‑accelerated on most ARM cores. For integrity purposes, a simple Fletcher‑16 or even a 32‑bit XOR might be enough and save cycles. I need to measure the overhead on 128 KiB chunks; if it’s >1% of total CPU, consider switching.
· // TODO: Once the codec stabilises, produce a formal bitstream specification (like a mini RFC) so that third‑party implementations can exist. Right now, only my Rust code defines the format.

---

This document will be updated as I bang on the code. Expect revisions.
– solo dev, somewhere with too much coffee and a Rust compiler.

```
