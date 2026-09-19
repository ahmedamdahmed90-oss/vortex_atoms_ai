# PERF-02 — GGML A/B Bench: BLOCKED (logged honestly)

**Date:** 2026-05-13
**Gate:** Section 2 / GGML backend — **NOT passed** (blocked on host toolchain).
**Status:** Blocked → pause. Nothing fabricated, nothing claimed as measured.

## Blocker (verified, not assumed)

The candle-vs-ggml A/B benchmark (deliverable 2.4) requires compiling
`llama-cpp-2` with the `native` feature, which needs:

| Requirement | Check | Result |
|---|---|---|
| CMake ≥ 3.21 | `cmake --version` | **NOT FOUND** |
| MSVC C++ toolchain (cl) | `Get-Command cl` | **NOT FOUND** (needs vcvars + Build Tools 2022) |
| Free disk for build artifacts | `Get-PSDrive C` | **1.3 GB free** (llama.cpp build alone needs ~4–8 GB) |

Verified on this 4-core, NO-AVX2 host. Without a C toolchain + disk headroom,
`cargo build --features ggml` cannot link llama.cpp, so any ggml tok/s number
would be fabricated. Per project rule: **no fabricated numbers**, therefore the
A/B bench is recorded as blocked, not as "done".

## What IS done and green (verified)

- 2.1 `ggml` Cargo feature + `llama-cpp-2` optional dep + `DEVELOPER_GUIDE.md` ggml section
- 2.2 `InferenceBackend` trait + `CandleBackend` (full) + `GgmlBackend` (feature-gated; `load` returns a truthful "not yet wired" error when `ggml` is off)
- 2.3 `inference_backend` config field (`default_inference_backend()`) + `inference.backend` wired into `vortex.json`
- 2.5 Build safety: `cargo check --all-targets` → OK, `cargo check --no-default-features` → OK, `cargo clippy --all-targets -- -D warnings` → **0 warnings** (all GREEN, measured)
- `pub mod inference;` in `src/lib.rs:41`

## Contingency (2.6 — documented, not executed)

Fallback if a toolchain becomes available later: use ONNX Runtime (ORT,
`ort = "2.0.0-rc.12"` already in `Cargo.toml`), which ships prebuilt Windows
binaries via `download-binaries` — **no cmake/MSVC needed**. The
`InferenceBackend` trait is the seam: a third `OrtBackend` implements the same
two methods (`load`, `generate_stream`) without touching `llm_inference.rs`.

## Timestamp clause

No throughput number for ggml is recorded here because no truthful one exists
on this machine. Any future `tok/s` value must come from an actual A/B run on
a host with CMake + MSVC + ≥8 GB free, and must be timestamped.

## How to unblock

```powershell
# Install VS Build Tools 2022 (Desktop development with C++) → re-open shell
# Install CMake
winget install Kitware.CMake
# Free disk (need ≥8 GB)
cargo clean
# Then:
cargo build --features ggml --release
cargo test --features ggml
cargo run --bin vortex_api --release -- --simd-probe
```
</content>
