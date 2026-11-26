# Building byte-bpe: an honest benchmark against HuggingFace

Crate: <https://crates.io/crates/byte-bpe> (publication pending owner email verification on crates.io, 2026-11-25). Source: <https://github.com/ibraramin/bpe-rs>.

## Why

The portfolio thesis pins Rust × AI-infrastructure, and tokenizers are named explicitly as a scarce, demoable signal. A from-scratch byte-level BPE is small enough to finish, large enough to explain in an interview, and permanent once it has a crates.io URL and generated docs.rs API docs.

## What it does

The base alphabet is all 256 byte values, so every byte string is representable and `decode(encode(bytes)) == bytes` holds even for bytes absent from the training corpus. Training counts adjacent symbol pairs, merges the most frequent pair (deterministic tie-break: lowest ids), and repeats until the vocabulary reaches the requested size. Encoding applies the learned merges in priority order. A plain-text save/load format makes trained tokenizers portable.

Correctness is checked by unit tests, five integration tests and a proptest property over arbitrary byte strings up to 256 bytes. The test suite passes; the crate is warning-free under `cargo clippy -- -D warnings`.

## The benchmark, as measured

| Method | MiB/s (median, 10 samples) | Corpus | Vocab |
|---|---|---|---|
| byte-bpe encode | 1.372 (CI 1.366 to 1.377) | fixed ASCII, 235,520 bytes | 512 |
| HuggingFace `tokenizers` BPE encode | 7.806 (CI 7.710 to 7.892) | same bytes | 512 |

Hardware: AMD Ryzen 7 6800HS Creator Edition, x86_64, 16 logical CPUs, rustc 1.92.0. Command: `cargo bench --bench bpe_bench -- --warm-up-time 1 --measurement-time 3 --sample-size 10`.

## Tradeoffs and caveats

- **The result is unfavorable, and that is the point.** HuggingFace is about 5.7× faster. Our encoder rebuilds a merge-rank map and rescans the whole sequence on each pass, while the mature library uses optimized pretokenization and parallel-friendly structures. Publishing the raw number is more credible than quietly omitting the comparison.
- **The corpora are identical bytes but not identical tokenizations.** HuggingFace's trainer uses its own BPE procedure over Unicode scalars with a 256-byte initial alphabet; ours merges over raw bytes. The vocabularies are not token-for-token identical, so this measures end-to-end encode throughput under matched inputs and target size, not a controlled algorithm microbenchmark.
- **Training is corpus dependent.** No pretrained vocabulary ships; users must train or load a saved file.
- **SIMD and wasm are not in this milestone.** Both were explicitly optional in the session design. Adding them without a measured win would be scope creep; they remain documented future work.

## What is next

Owner verification of the crates.io email unlocks `cargo publish`, docs.rs, a tagged `v0.1.0` release and the CV line. A follow-up can then test whether feature-gated SIMD scoring narrows the gap, with the same benchmark harness deciding honestly.
