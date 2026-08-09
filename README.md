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
