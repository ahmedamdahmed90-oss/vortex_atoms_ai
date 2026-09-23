// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# ENGINEERING REVIEW — Vortex Atoms AI (staged execution, 2026-09-20)

> Stage-by-stage record: what was claimed, what was verified, what changed.
> Baseline: Stage 1 (156 lib tests). Final: 191 lib tests + 4 ignored benches.

## Per-stage verdicts

| Stage | Status | Changes |
|-------|--------|---------|
| 0 — Safety & Discovery | PASS | `docs/AGENT_INITIAL_ARCHITECTURE.md` (read-only stage) |
| 1 — Baseline | PASS | `docs/ENGINEERING_BASELINE.md`; 156/356 green, flakes recorded as PRE_EXISTING |
| 2 — Unicode/Error handling | PASS | char-boundary `chunk_text` + `truncate`; 6 tests. Audited safe: fastpath markers, arabic stemmer, llm_memory/structured/tools (find/ends_with/ASCII boundaries); `dashboard::clamp_string` already correct |
| 3 — Knowledge import | PASS | `ImportReport`, symlink/size/extension guards, API partial-failure surfacing; 6 tests |
| 4 — Embedding/retrieval | PASS | `FastHashEmbedder` honest rename (+alias), `VectorIndex` ANN seam, O(N·D) + measured numbers; 5 tests + 1 bench |
| 5 — RAG | PASS | `ContextBuilder` + UNTRUSTED delimiters + budget; `build_rag_context` compat shim; 5 tests |
| 6–7 — Atom | PASS | `src/atom.rs`, orchestrator metrics + observers, centralized budget; 8 tests |
| 8 — Inference | PASS | speculative gated to greedy (`is_greedy`), scope docs; 3 sampler tests |
| 9 — Concurrency | PASS | history-reset fix on 3 endpoints, `ApiState` model docs; no lock changes |
| 10 — Security audit | PASS | controls verified in code; no critical vuln; no changes needed |
| 11 — API/observability | PASS | inventory matches API_REFERENCE; TokenUsage is tokenizer-measured; doc notes added |
| 12 — 5-Kernel | PASS | contracts documented, `kernel_liveness()` probes; 2 tests |
| 13–14 — Perf/bench | PASS | 3 ignored benches + `docs/BENCHMARKS.md`; no blind optimizations |
| 15 — Docs sync | PASS | this file, ATOM_ARCHITECTURE.md, API statelessness notes |
| 16 — Final regression | PASS | below |

## Issues from prior reviews: disposition

| Claimed issue | Verdict |
|---------------|---------|
| Unsafe UTF-8 slicing in chunking | CONFIRMED → fixed (Stage 2) |
| Silent `unwrap_or(0)` import loss | CONFIRMED → fixed via ImportReport (Stage 3) |
| Hash embeddings called "neural" | CONFIRMED (naming) → renamed + documented (Stage 4) |
| Linear retrieval, no index | CONFIRMED → documented + measured + seam (Stage 4); no ANN (unjustified scale) |
| RAG prompt injection surface | CONFIRMED → trust boundary added (Stage 5) |
| No atom abstraction | CONFIRMED (missing) → built thin layer (Stage 6–7) |
| Speculative scope unclear | CONFIRMED → greedy-gated + documented (Stage 8) |
| Shared history cross-client leak | CONFIRMED → reset on 3 endpoints (Stage 9) |
| Missing kernel health signal | CONFIRMED (missing) → liveness probes (Stage 12) |
| Flaky `test_five_kernel_matrix_submit_ui_input` | PRE_EXISTING → untouched (already `#[ignore]`) |
| Firefox E2E flakes | PRE_EXISTING → untouched (KNOWN_ISSUES) |
| `cargo audit` upstream warnings | PRE_EXISTING → untouched (continue-on-error) |
| `/ws` + IKC per-session partitioning | ~~CONFIRMED limitation~~ → **done** (session-ify: IKC/tool/batch/agent fork ephemeral sessions; per-session cancel flag). Engine pool for concurrent execution → **done** (`EnginePool`, `performance.pool_size`). |
| GB-scale atom memory experiment | Out of scope for this box → designed in BENCHMARKS.md, not executed |

## Before / after metrics

| Metric | Before (Stage 1) | After (Stage 16) |
|--------|------------------|------------------|
| Rust lib tests | 156 | 191 (+35), 0 failed |
| Ignored benches | 0 | 4 |
| Frontend tests | 356 | 356 (untouched) |
| Clippy (`learner,tray`, `-D warnings`) | clean | clean |
| `cargo fmt --check` | clean | clean |
| Public API signatures | — | unchanged except additive (`ImportReport`, `ContextBuilder`, `VectorIndex`, atom types, liveness probes) |

## Remaining limitations / trade-offs

1. ~~WS/IKC streaming sessions share one engine~~ — **state isolation done**
   (history/sampler/cancel are per-session on IKC, tool, batch, agent, WS,
   HTTP). Engine **pool done** (`EnginePool` free-list + config epoch;
   `performance.pool_size` default 1, max 4 — each slot owns full weights).
2. ~~ANN retrieval not implemented~~ — **done** (IVF opt-in via
   `VectorStore::trained_ivf`; see future work #4).
3. Inference/memory numbers unmeasured here (no model on box; harness exists).
4. ARCHITECTURE.md §IKC sketch predates current code (pre-existing staleness, noted, not rewritten in this pass).
5. Eviction ties resolve nondeterministically (valid victim either way).

## Recommended future work

1. ~~Per-session inference engines~~ — **state isolation done** (session
   partition on every generate path + per-session cancel). ~~Engine pool
   for concurrent execution~~ — **done** (`EnginePool` free-list, config
   epoch lazy reload, `performance.pool_size` 1..=4).
2. GB-scale atom experiment on provisioned hardware (BENCHMARKS.md §design).
3. ~~Probabilistic speculative acceptance for sampling modes~~ — **done**
   (Levi accept + residual sampling; see CHANGELOG `[Unreleased]`).
4. ANN index behind `VectorIndex` when N warrants it — **done** (IVF in
   `llm_embed.rs`, opt-in via `VectorStore::trained_ivf`).
5. ~~`sampler` scratch `mem::take` optimization with tokens/sec proof~~ —
   **done** (`sample_with_scratch` non-greedy: `mem::take` + capacity
   restore; 113.1 samples/s @ vocab 32k measured, 3 tests + bench —
   see BENCHMARKS.md + CHANGELOG `[Unreleased]`).
