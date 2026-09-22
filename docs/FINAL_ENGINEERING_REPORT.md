// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# FINAL ENGINEERING REPORT — Vortex Atoms AI (staged execution, 2026-09-20)

## 1. Initial state

Single-crate Rust workspace (`vortex_atoms_ai` v0.2.0) + React/TS frontend.
5-Kernel Matrix runtime, GGUF/Candle inference, hash-fallback embeddings,
linear vector retrieval, knowledge fragment registry, bearer-auth API.
Baseline (Stage 1): 156 lib tests, 356 frontend tests, fmt/clippy clean.

## 2. Issues confirmed (by reading current code, not inherited claims)

1. `chunk_text` byte-slicing panics on non-ASCII + `usize` underflow when
   `overlap >= size` (+ `truncate()` same class).
2. `import_directory` silently dropped per-file failures (`unwrap_or(0)`).
3. `EmbeddingModel` (hash) implemented `NeuralEmbedder` — misleading name.
4. Retrieval O(N) undocumented, unmeasured, no ANN seam.
5. RAG context raw-concatenated into prompts — no trust boundary.
6. No atom abstraction, no centralized budget, no lifecycle metrics.
7. Speculative verifier (argmax-accept) ran under sampling modes.
8. Shared engine history leaked across clients on 3 endpoints.
9. No kernel liveness signal.
10. Integration test asserted the old RAG format (broke at Stage 5).

## 3. Issues already fixed before this task (untouched)

Flaky `test_five_kernel_matrix_submit_ui_input` (`#[ignore]`), Firefox E2E
flakes (KNOWN_ISSUES #1–#3), `cargo audit` upstream warnings, `dashboard`
`clamp_string` boundary handling, API auth/sandbox/body-limits, per-item
history clear in `batch_generate`.

## 4. Issues fixed (this execution)

| # | Fix | Files |
|---|-----|-------|
| 1 | Char-boundary chunking, saturating step, degenerate fallback | `knowledge_import.rs`, `bin/vortex_chat.rs` |
| 2 | `ImportReport` + symlink/size/extension guards + API surfacing | `knowledge_import.rs`, `llm_api.rs` |
| 3 | `FastHashEmbedder` rename (+alias), honest docs | `llm_embed.rs`, `llm_neural_embed.rs` |
| 4 | `VectorIndex` trait, O(N·D) docs + measured scaling | `llm_embed.rs` |
| 5 | `ContextBuilder`, UNTRUSTED delimiters, char budget | `knowledge_import.rs` |
| 6 | `src/atom.rs`, orchestrator metrics/observers, `ResourceBudget` | `atom.rs`, `knowledge_orchestrator.rs`, `five_kernel_matrix.rs`, `state.rs`, `lib.rs` |
| 7 | Speculative gated on `is_greedy()` + scope docs | `llm_inference.rs`, `speculative.rs`, `vortex_config.rs`, `llm_sampling.rs` |
| 8 | `clear_history` on generate/tool-call/batch + model docs | `llm_api.rs` |
| 9 | `kernel_liveness()`/`all_kernels_alive()` + contracts | `five_kernel_matrix.rs` |
| 10 | Integration test updated to delimited format | `tests/integration_tests.rs` |

## 5. Architecture changes

Additive only: `atom` module, `VectorIndex`, `ContextBuilder`,
`ImportReport`, liveness probes. No public API removed; no kernel count,
route, feature, or UI changed. The one intentional behavior change:
stateless generate/tool-call/batch (isolation fix, matches chat precedent).

## 6. Security changes

RAG trust boundary (delimiters), import traversal hardening, session
isolation fix, speculative distribution-soundness fix. Audit (Stage 10)
found no critical vuln; fail-closed defaults preserved throughout.

## 7. Performance changes

None to hot paths (no model available to measure; protocol forbids blind
optimization). Measured: chunk 24.1 MB/s, embed 241 µs, search ~4–5
µs/vector linear, RAG build 1.5 ms, atom ops µs-scale (debug build).

## 8. Atom changes

Thin observable layer (state machine + budget + metrics + observers) over
the existing orchestrator lifecycle. Migration table in ATOM_ARCHITECTURE.md.

## 9. Tests added (35 → 191 lib; +integration fix)

Stage 2: 6 unicode · Stage 3: 6 import · Stage 4: 5 retrieval · Stage 5: 5
RAG · Stage 6–7: 8 atom/lifecycle · Stage 8: 3 sampler · Stage 12: 2
liveness. Plus 4 ignored scaling benches. One self-caught flake in a new
test fixed (strictly ordered eviction counts).

## 10. Benchmark results

`docs/BENCHMARKS.md`: full table + O(N) confirmation + honest unmeasured
scope (inference/RSS/GB-scale need a provisioned bench machine).

## 11. Before / after metrics

| Suite | Before | After |
|-------|--------|-------|
| `cargo test --features learner --lib` | 156 | **191** (3× stable) |
| `--test integration_tests` | 96 + 1 ignored | **97** + 1 ignored |
| `--test unit_tests` | 47 | 47 |
| Frontend vitest | 356 | 356 |
| fmt / clippy `-D warnings` | clean | clean |
| E2E | 54 (CI) | not rerun locally (browsers); code paths touched are backend-only except none |

## 12. Remaining limitations

WS/IKC/tool/batch/agent paths are session-isolated (history, sampler,
temperature, cancel); weights/KV remain one shared serial engine (pool =
future work). No ANN gap (IVF opt-in done); inference/RSS unmeasured here;
ARCHITECTURE.md IKC sketch predates code; eviction ties nondeterministic
(valid victim either way).

## 13. Known trade-offs

- Delimited RAG context lengthens prompts slightly (security > tokens).
- `clear_history` on generate removes global "memory" some may have leaned
  on — but it was cross-client leakage, not a feature.
- Speculative decoding supports sampling modes via probabilistic (Levi)
  acceptance when `performance.speculative_enabled` (default off);
  greedy stays argmax-exact. See CHANGELOG `[Unreleased]`.

## 14. Recommended future work

Per-session engine **pool** for concurrent execution (state isolation
**done** — session partition everywhere); GB-scale atom experiment; ANN
behind `VectorIndex` (**done** — IVF opt-in); sampler scratch `mem::take`
with tokens/sec proof. Probabilistic speculative acceptance **done**
(Levi). Details in ENGINEERING_REVIEW.md + BENCHMARKS.md.
