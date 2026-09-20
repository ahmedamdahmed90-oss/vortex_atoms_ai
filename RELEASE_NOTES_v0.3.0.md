## What's Changed

### Security Hardening (SEC-01)
- **Encryption at Rest**: `at_rest` health flag + 5 integration tests
- **Transport Hijacking**: WS origin check, CSP/X-Frame-Options/No-store/HSTS headers
- **Kiosk Abuse**: `--read-only` flag blocks mutating admin paths
- **Multi-Instance Conflicts**: Single-instance mutex, exit code 2, tray notification
- **Disk Exhaustion**: `check_disk_space` guard (1024 MB default)
- **Request Flooding**: Per-route body caps (1MB/2MB/50MB)
- **Config Tampering**: `replace_cfg` validation rejects unsafe relaxation (422)
- **Metrics Leak**: `/v1/metrics` Prometheus + `/v1/admin/metrics` JSON
- **Session Pile-up**: Admin purge endpoint + startup retention
- **Supply Chain**: SBOM (715 components), provenance, SHA-256 sidecars

### Investor Data Room (FUND-01)
- `docs/data_room/facts.json` — single source of truth
- 10 documents (AR + EN) + SHA-256 manifest
- 10-slide pitch deck outline with speaker notes
- 12 hard Q/As with honest answers
- Generator, freshness checker, manual-numbers guard

### Code Hygiene (CODE-HYGIENE-01 + 02)
- 94 oxlint warnings fixed (0 remaining)
- `lint:strict` promoted to gate status
- 2 new unit tests

### Certified Numbers
| Gate | Result |
|------|--------|
| `cargo test --features learner --lib` | 156 passed, 0 failed |
| `cargo clippy --features learner,tray` | 0 errors |
| `vitest` | 356 passed |
| `e2e` | 54/54 (3 Firefox flakes on retry) |
| `lint:strict` | 0 errors, 0 warnings |

## Verification

```powershell
# Verify release integrity
./tools/verify_release.ps1 -Version "0.3.0"

# Verify facts are fresh
./tools/check_facts_fresh.ps1
```

## Attachments

| File | Description |
|------|-------------|
| `SHA256_MANIFEST.txt` | Checksums for all release artifacts |
| `SBOM.json` | Software Bill of Materials (CycloneDX 1.4, 715 components) |
| `provenance.json` | Build provenance with SHA-256 digest |
| `SEC01_LANDING.md` | Security landing report |

## Honesty Clause

Every claim in this release traces to a CI-verified number. Green CI = certified numbers. See [REPO_STATUS.md](docs/REPO_STATUS.md) for details.
