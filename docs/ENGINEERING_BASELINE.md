// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# ENGINEERING BASELINE — Vortex Atoms AI (2026-09-20)

> Stage 1. Commands reproduced locally on the maintainer machine.
> No code changed in this stage.

## Environment

- OS: Windows 10 IoT Enterprise LTSC 10.0.19044, x64
- CPU: Intel i5-2430M, 2 cores (no AVX2; SIMD AVX + SSE4.2 + SSE4.1)
- Rust: 1.96.1 | Cargo: 1.96.1
- Node: via frontend toolchain (npm workspaces, vite 8, vitest 4)
- GPU: none available for this run

## Baseline results (local, 2026-09-20)

| Check | Command | Result |
|-------|---------|--------|
| fmt | `cargo fmt -- --check` | PASS (exit 0) |
| check | `cargo check --features learner --tests` | PASS |
| lib tests | `cargo test --features learner --lib` | 156 passed, 0 failed |
| clippy | `cargo clippy --all-targets --features learner,tray -- -D warnings` | PASS (0 warnings) |
| frontend unit | `cd frontend && npm run test` | 22 files, 356 passed |
| oxlint | `npx oxlint --deny-warnings` | 0 warnings, 0 errors (76 files) |
| tsc | `npx tsc --noEmit` | PASS (no output) |
| E2E | `npm run test:e2e` | NOT run locally this stage (CI: 54/54, 3 Firefox flakes green-on-retry) |
| frontend build | `npm run build` | PASS (prior SYNC-02 run; chunk-size warning only) |

## Known failures / flakes (PRE_EXISTING, not introduced)

1. `test_five_kernel_matrix_submit_ui_input` (tests/integration_tests.rs:890):
   timing-sensitive `LogicExecuted` event assertion. Marked `#[ignore]`
   with comment. PRE_EXISTING flake; documented in SYNC-04 diagnosis.
2. Firefox E2E flakes (3): green on retry. Tracked in KNOWN_ISSUES.md +
   GitHub issues #1-#3 with `flaky` label. PRE_EXISTING.
3. `cargo audit`: upstream warnings (ttf-parser unmaintained, glib
   unsound, chacha20 yanked). `continue-on-error: true` in CI.
   PRE_EXISTING, not a code regression.
4. GitHub runner "No space left on device" (run 35516146408): infra
   transient, not code. PRE_EXISTING infra flake.
5. `cargo clippy --all-targets --all-features` blocked by `cudarc`/CUDA
   on this machine; canonical command uses `--features learner,tray`.
   PRE_EXISTING constraint.

## Warnings

- Vite chunk-size warning (>500 kB): informational only.
- Node 20 deprecation warnings in CI actions: infra, not code.

## Conclusion

Baseline is reproducible and green for all gates relevant to code health.
No new regression introduced (no code changed in Stage 0-1).

Stage: 1
Status: PASS
Changes: docs/AGENT_INITIAL_ARCHITECTURE.md, docs/ENGINEERING_BASELINE.md (docs only)
Tests: as table above
Benchmark: none in this stage (Stage 13-14)
Regressions: none
Next Stage: 2
