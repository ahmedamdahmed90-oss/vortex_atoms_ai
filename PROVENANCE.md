# Build Provenance

**Project:** Vortex Atoms AI v0.2.0  
**Date:** 2026-09-18  
**Compiler:** rustc 1.78+

## Build Environment

| Property | Value |
|----------|-------|
| Rust version | 1.78 (minimum) |
| Features | `learner`, `tray` |
| Build command | `cargo build --features learner,tray --release` |
| Test command | `cargo test --features learner --lib` |
| Clippy command | `cargo clippy --all-targets --features learner,tray -- -D warnings` |
| SBOM | `SBOM.json` (CycloneDX 1.4, 715 components) |

## Source Integrity

- `Cargo.lock` is committed and pinned — all 715 dependencies verified via SHA-256 checksums
- `cargo build` verifies every crate checksum against crates.io registry
- No `git` submodules; all dependencies resolved via Cargo registry

## Build Provenance Chain

1. `Cargo.toml` — declares direct dependencies and feature flags
2. `Cargo.lock` — pins all transitive dependencies with SHA-256 checksums
3. `cargo build` — compiles with `--release` and verifies all checksums
4. `cargo test` — runs 156 unit/integration tests
5. `cargo clippy` — enforces lint rules with `-D warnings`
6. `SBOM.json` — generated from `Cargo.lock` with all 715 components
7. Release binaries hashed via `tools/generate_sha256.ps1`

## Trusted Toolchain

- Rust toolchain installed via `rustup`
- `cargo` verifies crate signatures against the official crates.io index
- No external build scripts or arbitrary code execution during build

## Audit Trail

- `docs/audit.log` — records all admin actions (reload, rotate, sessions-purge, etc.)
- Every security-critical operation is logged with peer IP, action, and success/failure
