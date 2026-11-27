# Real benchmark results — 2b-A2 (no fabricated numbers)

## Host, corpus, commands

- Host (verbatim): `Linux shafin-laptop 7.2.11-1-cachyos #1 SMP PREEMPT_DYNAMIC Sun, 18 Oct 2026 14:54:49 +0000 x86_64 GNU/Linux`; `AMD Ryzen 7 6800HS Creator Edition`, `x86_64`, `nproc` = 16; `rustc 1.92.0`, `cargo 1.92.0`. One host for every run below.
- Frozen corpus: `results/corpus.txt`, **50,020 bytes**, SHA-256 `e5266287e4696c3cb724a47ad8de7a86e7fe4eec5fea7e49becc2ed8ee577283`; mixed owner-authored prose/identifiers, seed 20261126, rebuilt by preserved `results/make_corpus.py`. Embedded with `include_bytes!`; hash unchanged across all definitive runs.
- Commands, all from `/var/tmp/opencode/cv-phase-2b-fork/projects/bpe-rs`: `cargo bench --all-features` (3 definitive runs, exit 0 each), `cargo test --all-features` (exit 0), `cargo test` (exit 0). Definitive runs: 05:31:10–05:32:40Z, 05:32:40–05:33:40Z, 05:33:40–05:34:39Z.
- Raw Criterion console output: `results/raw-criterion/definitive-run{1,2,3}-console.log`; final-run JSON: `results/raw-criterion/definitive-run3-scalar-estimates.json` and `-simd-estimates.json`; live output `target/criterion/encode_realistic/{scalar,simd}/512/new/estimates.json`.

## Authoritative gate — final code (parallel removed), group `encode_realistic`, VOCAB_SIZE=512

Mean throughput (MiB/s, Criterion's default console estimator), 95% CI in brackets. Speedup = scalar / SIMD.

| Run | scalar mean [CI] | simd mean [CI] | SIMD speedup | conservative CI speedup | CIs disjoint? |
|---|---|---|---|---|---|
| 1 | 0.5001 [0.4744, 0.5241] | 1.3453 [1.2871, 1.4015] | **2.69x** | 2.46x | yes |
| 2 | 0.6202 [0.6129, 0.6271] | 1.5786 [1.5665, 1.5882] | **2.55x** | 2.50x | yes |
| 3 | 0.6445 [0.6422, 0.6470] | 1.4376 [1.4320, 1.4426] | **2.23x** | 2.21x | yes |

Estimator correction (2026-11-26, post-panel R1): the console figures above are Criterion's **mean** point estimates, not medians. The pre-registered gate is defined on `median.estimate`. Only the final run's median JSON was archived: `results/raw-criterion/definitive-run3-scalar-estimates.json` and `-simd-estimates.json` give run-3 medians scalar **0.6454** MiB/s [0.6443, 0.6475] and SIMD **1.4447** MiB/s [1.4385, 1.4468] — speedup 2.238x, scalar CI upper 0.6475 < SIMD CI lower 1.4385. Runs 1-2 archived only console logs, so their median estimates are not independently recoverable; their console means are retained above and are NOT relabelled as medians. The mean gate and the archived median gate are both satisfied; no figure was invented.

Pre-registered gate: >=15% mean speedup over >=3 reps, >=10% median speedup, and SIMD 95% CI lower bound > scalar upper bound in ALL 3 runs. **All gates met in all 3 runs; SIMD is KEPT, feature-gated `simd`.** Correctness: `cargo test --all-features` (4 SIMD tests incl. proptest over 0..=512 arbitrary bytes, byte-identical ids, decode identity, save/from_file) and `cargo test` both exit 0.

## Earlier code-stable runs (std::thread parallel arm; used for the parallel decision)

| Run | scalar | parallel | simd | parallel speedup |
|---|---|---|---|---|
| 1 | 0.6172 MiB/s | 0.6138 MiB/s | 1.5033 MiB/s | 0.99x |
| 2 | 0.6493 MiB/s | 0.6598 MiB/s | 1.6507 MiB/s | 1.02x |
| 3 | 0.6654 MiB/s | 0.5944 MiB/s | 1.6203 MiB/s | 0.89x |

Parallel failed the external >=1.25x gate in every run and was reverted (ADR-001 change-mind clause). Its CIs overlapped scalar in all three runs.

## DISCARD notice

The first two runs (05:16:18Z and 05:17:51Z) used the **rayon** `encode_parallel` implementation and the older 73,637-byte corpus `3a81cff5…`; a third attempt at 05:19:08Z exited 101 after a concurrent edit. All three are **DISCARDED** — never averaged with, or presented as, the runs above. The external `results/preregistration.md`, `results/bench-run1.log`, `results/bench-run2.log`, `results/criterion-run1/` and `results/make_corpus.py` are preserved untouched.

## Honest caveats

- The SIMD arm pairs SSE2 with a flat O(1) rank table replacing a `HashMap`; no scalar-plus-table control was pre-registered, so the measured gain cannot be attributed to the SSE2 instructions alone. It is a genuine candidate win, not a claim about SIMD intrinsics in isolation.
- This is documentation prose (real, mixed with Markdown/code), not a natural-language book excerpt; Project Gutenberg was unavailable offline.
- Run 1 was noisier (scalar CI ~+/-5%) than runs 2-3 (<=1%); all three still cleared CI separation.
