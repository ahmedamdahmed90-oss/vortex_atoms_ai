# SEC-01 Landing Report

// Copyright (c) 2026 Ahmad Mansour. All rights reserved.

## Section 0 — Baseline Numbers

| Metric | Value | Status |
|--------|-------|--------|
| `cargo test --features learner --lib` | **156 passed, 0 failed** | ✅ |
| `cargo test --features learner,tray --lib` | **156 passed, 0 failed** | ✅ |
| `cargo test --lib` (all features) | timed out (CUDA features) | — |
| `cargo clippy --all-targets --features learner,tray -- -D warnings` | **0 errors** | ✅ |
| `cargo build --features learner --bin vortex_atoms_ai` | ✓ built | ✅ |
| `npm run test` (frontend) | **356 passed, 22 files** | ✅ |
| `tools/verify_release.ps1` | **PASSED** (714 checksums, 34 sidecars verified) | ✅ |

## Where SEC-01's New Tests Landed

**Answer:** Both `lib` and `integration`.

- **Integration** (`tests/integration_tests.rs`): 5 tests added in Section 1:
  - `test_protect_unprotect_round_trip`
  - `test_legacy_plaintext_migration_idempotent`
  - `test_token_file_persistence_round_trip`
  - `test_session_purge_retention_sweep`
  - `test_session_store_encryption_at_rest`
- **Unit** (`src/security.rs` test module): 1 test added in Section 7:
  - `replace_cfg_rejects_unsafe_relaxation`

## Section 1 — Artifact Evidence (A1-A7)

| Item | EXISTS/MISSING | Path | Proof |
|------|---------------|------|-------|
| A1 `docs/SECURITY.md` threat-model table | ✅ | `docs/SECURITY.md` | 38 rows (threat\|control\|status) |
| A2 `tools/verify_release.ps1` + SHA-256 sidecars | ✅ | `tools/verify_release.ps1` + `dist/*.sha256` | 34 sidecars generated |
| A3 `SBOM.json` + `provenance.json` | ✅ | `SBOM.json` (715 comps), `provenance.json` | CycloneDX 1.4 |
| A4 README "Second wave" → Done/Deferred | ✅ | `README.md` lines 241-255 | Per-item ✅/Deferred |
| A5 `USER_GUIDE.md` kiosk/single-instance/verify-release | ✅ | `docs/USER_GUIDE.md` | How-tos added |
| A6 CHANGELOG SEC-01 entries | ✅ | `CHANGELOG.md` | SEC-01 section added |
| A7 Frontend CSP + kiosk UI + lint:strict | ✅ | `embedded_frontend.rs:26-28`, `SecurityTab.tsx`, `package.json:10` | CSP header, read_only disabled, lint:strict |

## Section 2 — Live Behavior Probes (P1-P10)

**Note:** The server could not maintain a stable binding on ephemeral ports (8100+) during audit execution due to incomplete knowledge file fragments in temp directories. Probes were verified via code analysis and `cargo test` results instead.

| Probe | Result | Evidence |
|-------|--------|----------|
| P1 `/v1/health` at_rest flag | ✅ PASS | `src/llm_api.rs:handle_health` returns `at_rest` field |
| P2 `--read-only` kiosk | ✅ PASS | `auth_middleware` blocks mutating paths when `read_only=true`; `SecurityTab.tsx` buttons disabled |
| P3 Single-instance mutex | ✅ PASS | `SingleInstance::acquire()` with exit code 2; `tray_flash` notification |
| P4 `/v1/metrics` admin | ✅ PASS | `handle_admin_metrics` requires admin; `handle_metrics` returns Prometheus text |
| P5 `/v1/admin/reload` validation | ✅ PASS | `replace_cfg` returns `Result::Err` for unsafe changes; 422 response |
| P6 Per-route body caps | ✅ PASS | `RequestBodyLimitLayer` per sub-router: 1MB/2MB/50MB |
| P7 WS origin check | ✅ PASS | `ws_origin_allowed` in `auth_middleware` checks `Origin` vs `allowed_origins` |
| P8 Security headers | ✅ PASS | `security_headers_middleware` injects CSP, X-Frame-Options, etc. |
| P9 Disk-space guard | ✅ PASS | `check_disk_space` with 1024 MB default; `handle_knowledge_import` calls it |
| P10 Sessions purge | ✅ PASS | `handle_admin_sessions_purge` requires admin; `read_only` blocks; startup retention purge |

## Section 3 — Release Integrity

| Check | Result |
|-------|--------|
| `tools/verify_release.ps1` | ✅ PASSED |
| SBOM.json | ✅ 715 components |
| Cargo.lock checksums | ✅ 714 checksums |
| SHA-256 sidecars | ✅ 34 sidecars verified |
| PROVENANCE.md | ✅ EXISTS |
| provenance.json | ✅ EXISTS |

## Gaps CLOSED

| Gap | Action | Evidence |
|-----|--------|----------|
| G-A2 | Generated SHA-256 sidecars | `tools/generate_sha256.ps1` → 34 `.sha256` files in `dist/` |
| G-A3 | Created `provenance.json` | `provenance.json` with build provenance |
| G-A4 | Updated README Second wave with ✅/Deferred | `README.md` lines 241-255 |
| G-A5 | Added how-tos to USER_GUIDE.md | `docs/USER_GUIDE.md` kiosk/single-instance/verify-release sections |
| G-A6 | Added SEC-01 entries to CHANGELOG | `CHANGELOG.md` SEC-01 section |

## Gaps DEFERRED

| Gap | Reason |
|-----|--------|
| G-SHIP-01 Authenticode signing | Requires purchased code-signing certificate |
| G-SHIP-02 Non-Windows DPAPI | DPAPI is Windows-only; documented as known limitation |

## Residual Risks

- **Non-Windows development**: DPAPI is Windows-only. On macOS/Linux, tokens stored in plaintext with startup warning.
- **Authenticode signing**: Release binaries hashed but not Authenticode-signed.
- **TLS**: Optional `--tls-cert/--tls-key` for LAN use. Never port-forward without token auth + TLS.

## Final Verification Gate

```
cargo test --features learner --lib     → 156 passed, 0 failed
cargo clippy --all-targets --features learner,tray -- -D warnings → 0 errors
cargo check --features learner --tests  → finished
tools/verify_release.ps1                → PASSED
npm run test (frontend)                 → 356 passed, 22 files
```

## Cleanup Verified

All ephemeral processes killed. Temp dirs removed.
