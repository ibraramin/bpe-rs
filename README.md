# byte-bpe

A from-scratch byte-level BPE tokenizer for Rust, benchmarked honestly against HuggingFace `tokenizers`.

[![CI](https://github.com/ibraramin/bpe-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ibraramin/bpe-rs/actions/workflows/ci.yml) [![crates.io](https://img.shields.io/crates/v/byte-bpe.svg)](https://crates.io/crates/byte-bpe) [![docs.rs](https://docs.rs/byte-bpe/badge.svg)](https://docs.rs/byte-bpe) ![license](https://img.shields.io/badge/license-MIT-green) ![rust](https://img.shields.io/badge/rust-1.92%2B-orange)

> Naming note: this package is published as `byte-bpe` because `bpe-rs` was already taken on crates.io (verified and recorded on 2026-11-25). The repository directory and GitHub project remain `bpe-rs`.

## Install

```sh
cargo add byte-bpe
```

Crates.io: <https://crates.io/crates/byte-bpe>
Repository: `bpe-rs`

## Quick start

```rust
use byte_bpe::Tokenizer;

fn main() {
    // Train on any byte corpus. The base alphabet is all 256 byte values.
    let corpus = b"the quick brown fox jumps over the lazy dog; \
                   the quick brown fox jumps over the lazy dog;";
    let tokenizer = Tokenizer::train(corpus, 512);

    let ids = tokenizer.encode(b"quick brown fox");
    let bytes = tokenizer.decode(&ids).expect("encode output always decodes");
    assert_eq!(bytes, b"quick brown fox");

    // Persist and reload a trained tokenizer.
    let path = std::env::temp_dir().join("byte-bpe-example.bin");
    tokenizer.save(&path).expect("save");
    let reloaded = Tokenizer::from_file(&path).expect("load");
    assert_eq!(reloaded.encode(b"quick brown fox"), ids);
    let _ = std::fs::remove_file(&path);
}
```

## Demo

![byte-bpe terminal demo](demo.gif)

A 24-second terminal recording showing train, encode/decode, `cargo test` and a Criterion run. Source: `examples/demo.rs`.

## Architecture notes

- Byte-level alphabet: the initial vocabulary contains all 256 possible byte values, so every byte string is representable and `decode(encode(bytes))` is the identity.
- Merge priority: training counts adjacent symbol pairs over the corpus and repeatedly merges the most frequent pair, with deterministic tie breaking, until the requested `vocab_size` is reached. Learned merge rules carry an explicit priority and encoding applies them in that order.
- Serialization: `save` writes a plain-text snapshot. The first line is the vocabulary size `N` as a decimal integer, followed by exactly `N` lines holding each token's bytes as lowercase hexadecimal (a blank line denotes empty bytes), then one `LEFT RIGHT` line per merge rule using decimal token ids in priority order. `from_file` parses that layout and rebuilds the exact same id space, reporting `InvalidData` on a malformed file.
- No runtime dependencies are needed to encode once a tokenizer is trained.

## Benchmarks

Run with `cargo bench` or `cargo bench --all-features`. `benches/bpe_bench.rs` defines two Criterion groups, both measured on one local machine only (AMD Ryzen 7 6800HS Creator Edition, x86_64, 16 logical CPUs, stable rustc 1.92.0):

- `encode_throughput` — the original comparison against HuggingFace `tokenizers` BPE on a deterministic in-memory ASCII fixture, 235,520 bytes (512 repetitions of one paragraph), with the same target vocabulary size (512) on both sides. That fixture is synthetic; see Comparison settings below for the exact HuggingFace configuration. Its historical run was recorded 2026-11-25 with `cargo bench --bench bpe_bench -- --warm-up-time 1 --measurement-time 3 --sample-size 10`; no results artifact under `results/` records those numbers, so none are quoted here. This group is separate from `encode_realistic` and the two methodologies must not be merged.
- `encode_realistic` — the pre-registered experiment. It encodes one frozen corpus, `results/corpus.txt` (50,020 bytes, SHA-256 `e5266287e4696c3cb724a47ad8de7a86e7fe4eec5fea7e49becc2ed8ee577283`), made of owner-authored prose and identifiers and rebuilt by `results/make_corpus.py`. The bytes are compiled into the benchmark with `include_bytes!` so the measured input cannot drift, and `VOCAB_SIZE = 512` on every arm.

`encode_realistic` has two arms over identical bytes: `scalar` (the unchanged default single-threaded `Tokenizer::encode`) and `simd` (`Tokenizer::encode_simd`, an SSE2 candidate with a flat O(1) rank table, compiled only under the opt-in `simd` feature on x86_64). The pre-registered rule — at least three full `cargo bench --all-features` runs, a >=15% mean speedup, a >=10% median speedup, the SIMD 95% CI lower bound strictly above the scalar upper bound in all runs, plus byte-identical ids, decode identity and save/load correctness gates — is recorded in `results/preregistration.md` (external, 05:18:50Z), `results/preregistration-orchestrator-051549.md` (restored original) and `results/preregistration-reconciliation.md` (timeline and verdict).

Three definitive `cargo bench --all-features` runs on the final code (2026-11-26, 05:31:10–05:34:39Z, exit 0 each) gave these Criterion console (mean-estimator) throughputs in MiB/s, 95% CI in brackets:

| Run | scalar mean [95% CI] | SIMD mean [95% CI] | SIMD speedup | CIs disjoint |
| --- | --- | --- | --- | --- |
| 1 | 0.5001 [0.4744, 0.5241] | 1.3453 [1.2871, 1.4015] | 2.69x | yes |
| 2 | 0.6202 [0.6129, 0.6271] | 1.5786 [1.5665, 1.5882] | 2.55x | yes |
| 3 | 0.6445 [0.6422, 0.6470] | 1.4376 [1.4320, 1.4426] | 2.23x | yes |

Criterion's default console estimator is the **mean**; the pre-registered gate is defined on the median. Only run 3's median JSON was archived: true medians are scalar 0.6454 MiB/s [0.6443, 0.6475] and SIMD 1.4447 MiB/s [1.4385, 1.4468] (2.238x, disjoint CIs). Runs 1-2 have only console logs, so their medians are not separately claimed. Both the mean gate and the archived median gate are met; details in `results/benchmark-results.md`.

All gates were met in all three runs, so the `simd` feature is kept, still opt-in (`default = []`). `cargo test` and `cargo test --all-features` both exit 0. Full numbers, commands and provenance are in `results/benchmark-results.md`; raw Criterion console logs and JSON are in `results/raw-criterion/`. Earlier runs on superseded code or corpus are discarded and are not averaged with these figures.

A third, thread-parallel candidate was reverted under ADR-001's change-mind clause: it measured 0.99x / 1.02x / 0.89x against scalar with overlapping CIs. No thread-parallel dependency or parallel encode method exists in the final code.

### Comparison settings

The HuggingFace arm of the `encode_throughput` group uses `tokenizers` 0.21 with these exact settings, documented in `benches/bpe_bench.rs`:

- model: `tokenizers::models::bpe::BPE::default()`
- trainer: `BpeTrainer::builder().vocab_size(512).initial_alphabet((0u8..=255).map(char::from).collect::<HashSet<char>>()).show_progress(false).build()`
- special tokens: none
- normalizer, pre-tokenizer and decoder: none, plain BPE over Unicode scalar values
- training: the corpus is fed as whitespace separated words through `Trainer::feed`, then `Trainer::train` fits the BPE model

Version note: `tokenizers` 0.21 removed the older `Tokenizer::train_from_iterator` method, and the dynamically typed `tokenizers::Tokenizer` uses an internal `ModelWrapper`. The benchmark therefore trains the `BPE` model directly through the `Trainer` trait and wraps the trained model in a `Tokenizer`. Both sides see the same corpus bytes and the same vocabulary target.

## Limitations

- No pretrained vocabulary is shipped. You must train on your own corpus with `Tokenizer::train` or load a previously saved file with `Tokenizer::from_file`.
- Training is corpus dependent. Merge rules only reflect the bytes and pair frequencies of the text used to train, so encode quality and token counts will vary across domains.
- The `simd` feature is opt-in and is NOT part of the published v0.1.1 crate; there was no version bump or publish for it.
- The measured gain couples SSE2 with a flat O(1) rank table that replaces the `HashMap`; no scalar-plus-table control was pre-registered, so the win is not attributable to SSE2 instructions alone.
- Benchmark methodology caveats: `encode_throughput` uses synthetic ASCII text, not a real downstream dataset, and its figures must not be merged with `encode_realistic`. The pre-registered `encode_realistic` corpus is real owner-authored prose with a fixed SHA-256, but it is Markdown-and-code documentation, not a natural-language book excerpt. The HuggingFace model uses its own BPE training procedure and an alphabet of byte values represented as Unicode scalar values, so vocabularies are not token for token identical. Every measurement is from one local machine (AMD Ryzen 7 6800HS, 16 CPUs, rustc 1.92.0); results depend on CPU, compiler version and corpus size. No hiring-outcome claim is made.
- WebAssembly is not included in this milestone. A wasm target is a possible future addition.

## Project status

- Session design: `docs/sessions/phase-2/ai.md`
- Pre-registration, verdict and results: `results/preregistration.md`, `results/preregistration-orchestrator-051549.md`, `results/preregistration-reconciliation.md`, `results/benchmark-results.md`, the frozen `results/corpus.txt` and its generator `results/make_corpus.py`; raw Criterion output in `results/raw-criterion/`.
- Built and locally verified 2026-11-25: core library, proptest/edge-case tests, criterion benchmark vs HuggingFace, MIT LICENSE, README and CI. SIMD experiment finalized on final code 2026-11-26.
- Source and tagged `v0.1.1` release: <https://github.com/ibraramin/bpe-rs> (v0.1.0 remains as the initial historical tag).
- Published 2026-11-26: <https://crates.io/crates/byte-bpe> (v0.1.1) with generated API docs at <https://docs.rs/byte-bpe/0.1.1/byte_bpe/>.
- Earlier publish attempts were rejected until the owner verified the crates.io account email; the tagged GitHub releases remain the citable fallback. The 24-second demo GIF is above.

## License

MIT. See [LICENSE](LICENSE).
