// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# BENCHMARKS — Vortex Atoms AI (Stage 13–14)

> Measured 2026-09-20 on Intel i5-2430M (2 cores, no AVX2), Windows 10,
> **debug build** (release will be faster), machine under normal load.
> Rerun: `cargo test --features learner --lib bench_ -- --ignored --nocapture`
>
> Build flags (2026-09-21): the repo no longer pins `-C target-cpu=native`
> (see `.cargo/config.toml`). It crashed rustc with SIGILL on virtualized CI
> runners and tied release binaries to the build CPU. Runtime SIMD dispatch
> via `perf_topology` + pulp/gemm is unaffected. For a local max-perf build:
> `RUSTFLAGS="-C target-cpu=native"`.

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

## IVF approximate index (measured 2026-09-21, same machine, debug)

Hand-rolled spherical k-means IVF behind `VectorIndex` (zero new deps):
nlist=64, nprobe=8, D=64, k=10, clustered synthetic data (16 blobs).
Recall@10 vs exact linear ground truth. Rerun:
`cargo test --features learner --lib bench_ivf -- --ignored --nocapture`

| N | train (one-time) | recall@10 | IVF µs/q | linear µs/q | speedup |
|---|------------------|-----------|----------|-------------|---------|
| 1000 | 3.5s | 1.000 | 353 | 2333 | 6.6× |
| 5000 | 16.9s | 1.000 | 1750 | 12442 | 7.1× |
| 12000 | 40.6s | 1.000 | 4330 | 33331 | 7.7× |

Regression floor in-tree: recall@10 ≥ 0.95 (`ivf_recall_on_clustered_data`).
Crossover is below N=1000 — IVF wins everywhere measured. Default backend
stays linear (zero behavior risk); opt in via `VectorStore::trained_ivf`
when N or latency budget demands it.

Honest limits: recall measured on CLUSTERED data, where partitions work.
`FastHashEmbedder` vectors are near-orthogonal sparse — every centroid is
~equidistant there, so IVF recall on hash embeddings will be poor. ANN is
built for the neural-embedding future (`neural-embed`/OrtEmbedder), where
vectors actually cluster. Train cost is debug-build time; release is far
faster (train is offline, queries are what matter).

## Stage 13 profile notes (no blind optimization)

- No inference hot-path changes made: no model was available on this
  machine to measure generation, and the protocol forbids unmeasured
  "optimizations".
- Identified opportunity: `VortexSampler::sample_with_scratch`
  cloned the scratch buffer into a Tensor per token on the non-greedy path
  (`Tensor::from_vec(scratch.clone(), …)`). **Done (2026-09-23):**
  `mem::take` moves the buffer into the Tensor (no vocab memcpy); capacity
  is `reserve`d after the Tensor drops. Measured below.
- ` batch_generate` already clears history per item (verified Stage 9).

## Sampler non-greedy path — mem::take (measured 2026-09-23, debug)

Rerun: `cargo test --features learner --lib bench_sample_with_scratch -- --ignored --nocapture`

| Metric | Result |
|--------|--------|
| samples (vocab=32000, temp=0.8, debug) | 2000 in **17.69s** → **113.1 samples/s** |
| scratch capacity after bench | **32000** (restored, no shrink to 0) |
| old path cost removed | one `Vec<f32>` clone (~128 KB) per sample |
| greedy path | unchanged (argmax on scratch, no Tensor) |

Tests: `sample_with_scratch_nongreedy_restores_capacity`,
`sample_with_scratch_nongreedy_returns_valid_token`,
`sample_with_scratch_greedy_matches_argmax` (in-tree regression floor).

## Inference (measured 2026-09-21, closes the gap above)

Machine: Intel i5-2430M (Sandy Bridge, 2C/4T, no AVX2), Windows 10.
Model: Qwen2.5-0.5B-Instruct Q4_K_M (mmap). Binary: published v0.2.0
release (portable build, `target-cpu=native` removed — see note at top).

| Request | Result |
|---------|--------|
| `POST /v1/generate` "Hello", max_tokens=2 | 200 in **264.7s**, `text:"Hello!"` |
| `POST /v1/generate` "Hello again", max_tokens=2 | 200 in **265.7s** (no prefix-KV win — different prompt) |
| `POST /v1/embeddings` "Hello world" | 200 in **188ms** (hash path, not neural) |
| Same generate on **debug** build | did not finish within 120s / 600s timeouts |

Conclusion: the loop is correct (bounded, EOS + max enforced) — cost is
almost entirely **prefill of the full templated prompt** (system + RAG +
history, ~hundreds of tokens) at ~0.25s/forward-pass in release on this
CPU. Debug is 10×+ slower (appears hung; it is not). Practical guidance:
run the release binary, keep prompts short, use small max_tokens, or use
the `eco` (SmolLM2-135M) tier on weak hardware. `tokens_per_second: 0.0`
in responses is a hardcoded placeholder (pre-existing gap).

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
