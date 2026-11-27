# Pre-registration — 2b-A2 Criterion run and conditional SIMD decision

**Written (UTC): 2026-11-26T05:15:49Z. First `cargo bench` invocation: NOT YET RUN at
the time of writing.** This file was created before any benchmark command was
executed. No git commit was made (the work-burst forbids commits); this UTC
timestamp and the corpus hash below are the pre-registration evidence.

## Hardware (recorded verbatim, one host for all runs)

- `uname -a`: `Linux shafin-laptop 7.2.11-1-cachyos #1 SMP PREEMPT_DYNAMIC Sun, 18 Oct 2026 14:54:49 +0000 x86_64 GNU/Linux`
- `/proc/cpuinfo` `model name`: `AMD Ryzen 7 6800HS Creator Edition`
- Architecture `x86_64`; `nproc` = **16 logical CPUs**; SSE2 baseline (AVX2 flag present but the candidate below uses SSE2 only).
- Toolchain: `rustc 1.92.0 (ded5c06cf 2025-12-08)`, `cargo 1.92.0 (344c4567c 2025-10-21)`.

## Corpus (non-synthetic, committed, frozen before the run)

- File: `results/corpus.txt`, **73,637 bytes**, 1,112 lines.
- SHA-256: `3a81cff56a99472fd1ace9bcb6222f55fb1c1706c3a9ae031e14ddd4a2c4631e`.
- Provenance: verbatim concatenation of **this repository's own owner-authored
  documentation**, in order: `README.md`, `WRITEUP.md`, `docs/corporate-gameplan.md`,
  `docs/recommendations.md`, `docs/activeContext.md`,
  `docs/sessions/phase-2/{scope-decision,scope-expansion,ai,web}.md`. No third-party
  or unlicensed text was copied. The bpe-rs files are MIT (see `LICENSE`); the
  remaining files are the owner's own writing. Contains 345 non-ASCII UTF-8 bytes
  (e.g. `×`, `←`, `—`), so the corpus is not a synthetic ASCII-only fixture.
- Caveat recorded in advance: this is real documentation prose mixed with Markdown
  and code, not a natural-language book excerpt. Project Gutenberg was down for
  maintenance when the corpus was sourced, so no public-domain book text could be
  fetched offline; the project's own docs are explicitly allowed by the task.
- In addition to this corpus, the pre-existing in-code synthetic ASCII fixture
  remains as a separate `encode_throughput` group; it is **not** used for the
  SIMD decision.

## Metric and decision rule (pre-registered, conjunctive)

- Metric: Criterion **median encode throughput (MiB/s)** for the `simd` arm versus
  the `scalar` arm **in the same `encode_realistic` benchmark group**, over
  **identical bytes** from `results/corpus.txt`, `VOCAB_SIZE = 512`,
  `Throughput::Bytes`. Raw Criterion output under `target/criterion/encode_realistic/`.
- Repetitions: **3 independent full `cargo bench --all-features` runs** on the host above.
- Primary threshold (verbatim): **">=15% mean speedup across >=3 repetitions"**.
- Pre-registered refinements from the independent oracle (folded in here before
  any benchmark run, not post-hoc): require **>=10% median speedup** AND the
  **SIMD 95% CI lower bound strictly greater than the scalar 95% CI upper bound
  in ALL 3 runs**.
- All of the above are conjunctive gates. If **any** gate is missed in **any** run,
  the outcome is negative.

## Correctness gate (all must pass before a win may be claimed)

1. `cargo test --all-features` exit 0 and `cargo test` (default features) exit 0.
2. Proptest over arbitrary bytes in both test binaries.
3. Byte-identical token ids: `encode_simd(bytes) == encode(bytes)` and
   `encode_parallel(bytes) == encode(bytes)` for empty, unicode multi-byte,
   all-256-byte and large-corpus inputs.
4. `decode(encode(bytes)) == bytes` identity for every alternate path.
5. `save` / `from_file` round-trip preserves vocab and encodings.

## Falsification rule (binding)

If the pre-registered win is **not** measured on every gate above, **remove the
`simd` feature and the SIMD candidate** and document the negative result under
`results/`. No SIMD claim may be made from a post-hoc subset of runs, a different
metric, or a synthetic-only corpus. Ground for the strict gate: the earlier Phase 4
experiment measured AVX2 matmul *slower* than scalar at 128x128x128, byte-bpe's
stated value is a small dependency-free baseline, and HuggingFace is already
~5.7x faster — a small or noisy win does not close the headline gap.

## Order of events (to be completed after the run, without changing the criteria above)

- Pre-registration written: 2026-11-26T05:15:49Z.
- First bench run: _recorded in `results/benchmark-results.md` after execution._
