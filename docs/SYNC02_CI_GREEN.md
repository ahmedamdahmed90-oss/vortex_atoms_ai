# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# SYNC-02 Section 3: CI Local Pre-Flight Results

Generated: 2026-09-20

## CI Matrix

| Job | Command | Exit | Status |
|-----|---------|------|--------|
| Backend: fmt | `cargo fmt -- --check` | 0 | ✅ PASS |
| Backend: clippy | `cargo clippy --all-targets --features learner,tray -- -D warnings` | 0 | ✅ PASS |
| Backend: tests (learner) | `cargo test --features learner --lib` | 0 | ✅ PASS (156 passed) |
| Frontend: oxlint | `npx oxlint --deny-warnings` | 0 | ✅ PASS (0 warnings, 0 errors) |
| Frontend: lint:strict | `npm run lint:strict` | 0 | ✅ PASS |
| Frontend: type check | `npx tsc --noEmit` | 0 | ✅ PASS |
| Frontend: unit tests | `npm run test` | 0 | ✅ PASS (356 passed, 22 files) |
| Frontend: build | `npm run build` | 0 | ✅ PASS |
| Data Room: check_facts_fresh | `tools/check_facts_fresh.ps1` | 0 | ✅ PASS |
| Data Room: check_no_manual_numbers | `tools/check_no_manual_numbers.ps1` | 0 | ✅ PASS |
| Data Room: check_repo_refs | `tools/check_repo_refs.ps1` | 0 | ✅ PASS (after SHA update) |

## Notes

- `check_repo_refs.ps1` required facts.json SHA update (c80ebf6 → c892231) after SYNC-01 commit
- Docker and Security jobs not run locally (Docker requires Docker daemon, Security requires cargo-audit)
- All critical jobs green locally
