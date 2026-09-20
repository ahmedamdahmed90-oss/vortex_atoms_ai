// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# BENCHMARKS — Vortex Atoms AI (Stage 13–14)

> Measured 2026-09-20 on Intel i5-2430M (2 cores, no AVX2), Windows 10,
> **debug build** (release will be faster), machine under normal load.
> Rerun: `cargo test --features learner --lib bench_ -- --ignored --nocapture`

## Micro-benchmarks (measured)

| Benchmark | Result | Notes |
|-----------|--------|-------|
| `chunk_text` throughput | 140 KB → 322 chunks in 5987 µs (**24.1 MB/s**) | Stage 2 char-boundary chunking; Arabic mixed in |
| `hash_embed` (D=64, ~1 KB text) | **241 µs/embed** | FNV-1a lexical fallback, not neural |
| `linear_search` N=200 | 707 µs/query | D=64, top_k=5 |
| `linear_search` N=1000 | 4146 µs/query | ~5.9× for 5×N |
| `linear_search` N=5000 | 24671 µs/query | ~5.9× for 5×N — textbook O(N) |
| `rag_build` (322 chunks in store) | 1534 µs/build | embed + top-k + delimit + budget |
| `atom_select` (1000 registered) | 4939 µs/select | intent filter + cosine rank, top-3 |
| `atom_insert+touch` (4 KB) | 2 µs/op | registry ops are negligible |

Per-vector search cost ≈ 4–5 µs at D=64 (debug). Numbers vary run to
run (±2× observed between runs) — treat as order-of-magnitude, not spec.

## Scaling conclusion

- Retrieval is **O(N·D) linear**, confirmed empirically (Stage 4 + rerun).
  Realistic scale for the bundled knowledge (dozens of fragments,
  hundreds of chunks): single-digit ms per query. Fine as-is.
- The `VectorIndex` seam (Stage 4) exists for when N outgrows linear.
  No ANN dependency added — unjustified at current scale.
- Atom registry ops are sub-ms at 1000 atoms; lifecycle overhead is noise.

## Stage 13 profile notes (no blind optimization)

- No inference hot-path changes made: no model was available on this
  machine to measure generation, and the protocol forbids unmeasured
  "optimizations".
- Identified (not changed) opportunity: `VortexSampler::sample_with_scratch`
  clones the scratch buffer into a Tensor per token on the non-greedy path
  (`Tensor::from_vec(scratch.clone(), …)`). A `mem::take`/`replace` form
  would remove one vocab-size memcpy per token — left for a stage with a
  loaded model + tokens/sec harness to prove it.
- ` batch_generate` already clears history per item (verified Stage 9).

## NOT measured here (honest scope limits)

- **Inference**: TTFT, tokens/sec, total latency — require a downloaded
  GGUF model + warm engine. Harness exists (`/v1/bench` + PERF_REPORT).
- **Memory**: RSS / mapped-vs-resident under load — require a running
  server with model + knowledge. `MmapWeights` + prefault paths were
  audited by reading (Stage 8); no "never enters RAM" claim is made
  anywhere in code or docs.
- **GB-scale atom experiment** (10 MB → 10 GB datasets × 256 MB → 2 GB
  budgets, RSS/routing-accuracy/cache-hit matrix): designed, not executed —
  infeasible on this 2-core box without a model. Recommended as the first
  task on a properly provisioned bench machine:
  1. synthesize fragment corpora at 10 MB / 100 MB / 1 GB (µµ skip 10 GB
     until disk/RAM allow);
  2. register → select → load → evict under each `ResourceBudget`;
  3. record peak/avg RSS, routing latency, retrieval latency, load
     latency, hit rate;
  4. compare atom lifecycle vs naive load-all RAG on identical hardware.
- **Vortex vs in-memory RAG vs ANN**: same blocker (no model, small box).
  The `VectorIndex` trait makes the ANN arm a drop-in when it happens.

## Gate

- No significant regression: lib suite green (see Stage 16 report).
- Measured improvements: none claimed (correct — none were made blindly).
- Memory behavior: understood at the architecture level (mmap source of
  truth, per-tensor materialization, 256 MiB matrix budget centralized).
- CPU bottlenecks: retrieval linear (measured, acceptable); inference
  unmeasured (documented above, not guessed).
