# Pre-registration reconciliation — two records, one honest verdict

Two pre-registration files coexist. Neither was rewritten to favour the outcome;
the external file is preserved byte-for-byte and my original is restored verbatim
from the context in which it was authored.

## Timeline (all UTC, 2026-11-26)

| Time | Event |
|---|---|
| 05:15:49 | Orchestrator-lane pre-registration written: `preregistration-orchestrator-051549.md` (restored here). No bench had run. Gates: >=15% mean over >=3 reps; >=10% median; SIMD 95% CI lower > scalar upper in ALL 3 runs; correctness gate. |
| 05:16:18–05:17:32 | First `cargo bench --all-features` (exit 0), rayon `parallel` arm, 73,637-byte corpus `3a81cff5…`. |
| 05:17:51–05:19:01 | Second run (exit 0). External writer replaced `results/corpus.txt` at 05:18:32 with 50,020-byte `e5266287…` mid-run. |
| 05:18:28–05:18:58 | External writer added `make_corpus.py` and its own `preregistration.md` (05:18:50, 2 runs, parallel >=1.25x vs serial, SIMD >=1.5x vs parallel). Preserved untouched. |
| 05:19:08 | Third run failed (exit 101): a concurrent edit to `encode_parallel` introduced a compile error mid-build. |
| 05:19:58–05:23:25 | Three code-stable gating runs on the `std::thread::scope` parallel code and corpus `e5266287…`, all exit 0. File hashes identical before each run and after run 3. |
| after 05:23 | My `bench-preregistration.md` was deleted by the concurrent writer; restored verbatim as `preregistration-orchestrator-051549.md` on owner/orchestrator instruction. |

## What happened to the files

- `preregistration.md` (external, 05:18:50Z): **preserved unmodified**. Its "no
  cargo bench had been run by this agent" statement is agent-relative; globally,
  the first benchmark started at 05:16:18Z, before it was written.
- `preregistration-orchestrator-051549.md`: restored verbatim, including its
  original corpus hash `3a81cff5…` and 73,637-byte count. That snapshot was
  superseded by an external corpus edit at 05:18:32Z; the frozen bytes used by
  the gating and final runs are 50,020 bytes, SHA-256
  `e5266287e4696c3cb724a47ad8de7a86e7fe4eec5fea7e49becc2ed8ee577283`.
- The CI-separation and all-3-runs criteria in my record were written at
  05:15:49Z, **before the first benchmark and before the final runs**, so they are
  pre-registered, not post-hoc.
- Earlier rayon-version runs (05:16:18Z, 05:17:51Z) and the failed run (05:19:08Z)
  are **DISCARDED** — not averaged with, or presented as, the gating runs.

## Verdict

- **SIMD**: passes both protocols. The three code-stable runs measured median
  throughput ~2.44x, 2.54x, 2.44x vs scalar with disjoint 95% CIs, and ~2.45x,
  2.50x, 2.73x vs the parallel scalar path; correctness gates were green.
  Attribution caveat: the candidate pairs SSE2 with a flat O(1) rank table that
  replaces a `HashMap`; no scalar-plus-table control was pre-registered, so the
  gain cannot be attributed to the SSE2 instructions alone.
- **Parallel**: fails the external >=1.25x gate (~0.99x / 1.02x / 0.89x vs
  serial). Per ADR-001 change-mind clause ("benchmark shows no reproducible win,
  then revert"), the `parallel` feature, method, tests and bench arm were removed.
  The default single-threaded `encode` path is unchanged.
- Because the compiled code changed, three fresh definitive runs were executed on
  the final code; their measured numbers are in `benchmark-results.md` and govern
  the final SIMD keep/drop decision.
