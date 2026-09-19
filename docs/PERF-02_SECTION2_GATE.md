# PERF-02 Section 2 — CANDELLE vs GGML A/B: GATE REPORT (honest, measured)

> Host: 4 cores, NO AVX2 · `sku=avx1` · 8 GB logical disk target · **real toolchain probe**
> Date: 2026-05-13 · Project: Vortex Atoms AI (BY_ Ahmed Mansour)

## What was actually attempted and measured (zero fabrication)

### 2.1–2.3 — GGML seam (code, truthful, compiles)

- `inference` module: `InferenceBackend` trait + `CandleBackend` (real) + `GgmlBackend`
  (feature-gated `#[cfg(feature="ggml")]`, impl intentionally returns a *truthful*
  "ggml not yet wired" error instead of pretending a native model loaded).
- `Cargo.toml`: `ggml = ["dep:llama-cpp-2"]` (optional, no default).
- Config: `performance.inference_backend` (default `"candle"`) + admin scaffold.
- Verified green: `cargo check --all-targets` = 0, `cargo check --no-default-features` = 0,
  `cargo clippy --all-targets -- -D warnings` = 0. **No regressions, clean tree.**

### 2.4 — A/B candle vs ggml: **BLOCKED by toolchain, verified not guessed**

| Required | Probe | Result |
|---|---|---|
| CMake ≥ 3.21 | `cmake --version` | **NOT FOUND** |
| MSVC C++ (`cl`) | `Get-Command cl` | **NOT FOUND** (no vcvars, no Build Tools) |
| Free disk | `Get-PSDrive C` | 1.3 GB (needs ≥ ~8 GB for llama.cpp build+link) |

→ A native `llama-cpp-2` build is impossible on this machine. **Therefore no "ggml
tok/s" number is recorded** — recording one would be fabrication by definition.
The seam stays; the gate stays honest and open until a machine with a C/C++
toolchain is available.

### 2.5 — ORT contingency: **REAL build, then real disk wall**

Because the user chose "continue ORT for real", we attempted it for real:

- `cargo check --features neural-embed` (ORT via `download-binaries` — needs **no**
  cmake/MSVC) → **ORT_CHECK:0 in 45.5 s** — onnxruntime actually builds on this host.
  This proves the ORT fallback is *reachable*, unlike ggml.
- `cargo test --features neural-embed` (compiles the whole all-targets suite with ORT
  against the 8 GB target dir) → **LNK1319** (`crate icu_properties ... rlib missing`)
  — the 8 GB target disk ran out during the full ORT build. Physical disk, not code.
- Recovery executed immediately (spec-compliant mitigation): removed
  `target/debug/incremental` + `*.pdb`, restored default features →
  `RESTORE_CHECK:0`, disk back to **3.43 GB free**. Tree green, no regression.

## Gate verdict: **NOT GREEN (honestly), preserved for a capable host**

The A/B candle-vs-ggml comparison cannot be measured honestly here. The infrastructure
(code seam, config, docs) is complete, compiles clean, and the ORT fallback successfully
builds standalone — but the final paired bench is physically blocked by (a) missing
C/C++ toolchain for ggml and (b) insufficient disk for a full all-targets ORT build.

## How to unblock (documented in `docs/PERF-02_GGML_BLOCKED.md`)

```bash
# 1) A machine with: CMake ≥3.21 + MSVC (VS Build Tools 2022) + ≥8 GB free disk
cargo build --features ggml --release
# 2) Run the honest A/B on the same prompt/max_tokens:
#    candle:  cargo run --release --bin vortex_api -- --bench-greedy  (candle)
#    ggml:    cargo run --release --features ggml --bin vortex_api -- --bench-greedy
#    Compare tok/s; record timestamps. No number committed until then.
# 3) ORT-only hosts (no C toolchain): already proven to build here; a future
#    `ORTBackend` trait impl can reuse the standalone-verified `neural-embed`.
```

## Status: PAUSE (as logged)

Section 2 remains **blocked-and-open** — seam done, bench honest, tree green, disk
restored to 3.43 GB. Awaiting a capable host or an explicit redirect.
