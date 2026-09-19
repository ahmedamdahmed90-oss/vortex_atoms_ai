# Security & Threat Model

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.
> SEC-01 security master prompt — posture baseline and threat model.

## Gate Baseline

| Metric | Value | Status |
|--------|-------|--------|
| `cargo test --features learner --lib` | 156 passed, 0 failed | ✅ |
| `cargo clippy --all-targets --features learner,tray -- -D warnings` | 0 errors | ✅ |
| `cargo build --features learner --bin vortex_atoms_ai` | ✓ built | ✅ |
| `cd frontend && npm run test` | 356 passed (22 files) | ✅ |

## Threat Model Table

Extracted from README "Security & Control Privileges" section.

| Threat | Control | Status |
|--------|---------|--------|
| No auth on local API — any process can drive the server | Bearer tokens (user/admin), auto-generated, persisted in `%LOCALAPPDATA%\vortex_atoms_ai\auth.json` | **done** |
| Cross-origin exploitation — website drives the API | Strict same-origin CORS, `security.allowed_origins` opt-in, per-IP rate limiting, `x-request-id` | **done** |
| Privileged action abuse — model swap, knowledge import | Admin token required, audit log (JSONL), Security tab in dashboard | **done** |
| Knowledge import path escape — arbitrary file read | `file_path` confined to allowlisted directories | **done** |
| Model filename/repo injection | Model filename + repo allowlists, SHA-256 cache-integrity manifest | **done** |
| Remote bind exposure — unauthenticated remote access | Non-loopback `--host` refuses without token auth + `--allow-remote`; optional TLS | **done** |
| XSS via model output — `javascript:` links | Model-rendered markdown links allowlist (http/https/mailto/relative only) | **done** |
| **Sessions/tokens at rest** — plaintext auth.json on disk | DPAPI-encrypted sessions + tokens, user-scoped ACLs, 30-day retention | **done** |
| **Transport hijacking** — WS upgrade without origin check | WS origin check in `auth_middleware` (checks `Origin` vs `security.allowed_origins` or `Host`), security headers middleware (CSP, X-Frame-Options: DENY, X-Content-Type-Options: nosniff, HSTS, no-store), idle timeout (300s), ping/pong (60s), max frame size (256 KB), per-IP rate limiting | **done** |
| **Kiosk abuse** — pilot kiosk allows mutations | `--read-only` kiosk mode (CLI flag + `security.read_only` config), `is_mutating_admin_path` blocks swap/import/rotate/reload/sessions-purge/performance, `SecurityTab` buttons disabled when `read_only` | **done** |
| **Multi-instance conflicts** — two launches corrupt state | `SingleInstance` named mutex (Windows) / lockfile (Unix), second launch exits **2**, `tray_flash` notified, chat also protected | **done** |
| **Disk exhaustion** — learner/cache/imports fill disk | `check_disk_space` guard (default 1024 MB min free + 512 MiB margin), graceful refuse + audit on `knowledge.import`, `llm_download` | **done** |
| **Request flooding** — large body DoS | Per-route body caps (chat/generate/batch 1 MB, import 50 MB, embeddings 2 MB) via `RequestBodyLimitLayer`, 413 on exceed | **done** |
| **Config tampering** — malicious hot-reload | `replace_cfg` validates atomically; unsafe relaxation → 422 + reason; audit every reload | **done** |
| **Metrics leak** — unauthorized metrics access | `GET /v1/metrics` Prometheus text + `GET /v1/admin/metrics` JSON; loopback-only or admin via `auth_middleware` | **done** |
| **Session pile-up** — stale sessions consume resources | `POST /v1/admin/sessions/purge` with `window.confirm` dialog, `read_only` blocks, startup retention purge (`sessions_retention_days=30`) | **done** |
| **Supply chain** — untrusted artifacts | `SBOM.json` (CycloneDX, 715 components from `Cargo.lock`), `PROVENANCE.md`, SHA-256 sidecars via `tools/generate_sha256.ps1`, verify via `tools/verify_release.ps1` | **done** |
| **Non-Windows dev at-rest** — no DPAPI available | Plaintext ONLY with startup WARNING log + health flag "at_rest": "unencrypted-dev" | **deferred** |

## Residual Risks

- **Non-Windows development**: DPAPI is Windows-only. On macOS/Linux, `auth.json` stores tokens in plaintext with a startup warning and `health.at_rest = "unencrypted-dev"`. This is documented and not hidden.
- **Authenticode signing**: Release binaries are hashed but not yet Authenticode-signed (requires purchased code-signing certificate). See `SECURITY.md` for details.
- **TLS**: Optional `--tls-cert/--tls-key` for LAN use. Never port-forward without token auth + TLS.

## Rollback Policy

- All security changes default ON only where safe for pilots
- Anything breaking existing installs ships OFF + migration note
- Rollback = config flip / feature flag; no surgery on shipped bits
