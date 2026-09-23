---
title: Repository Status
---

# REPO-01 — Repository Status

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.

**العربية:** [REPO_STATUS_AR.md](./REPO_STATUS_AR.md)

## What This Repo Is

This public repository is a **release mirror** — it contains squashed snapshots starting from v0.2.0. The full development history is private.

## Why 2 Commits?

The initial push was squashed into 2 commits to keep the public history clean:
1. **Initial commit** — full codebase snapshot
2. **CI/CD fixes** — all CI pipeline corrections

Future releases will each be a single squashed commit with a tag.

## How CI Certifies Numbers

Every push triggers the full CI gate:
- **Backend**: `cargo test` (225 tests), `cargo clippy` (0 warnings)
- **Frontend**: `vitest` (356 tests), `e2e` (54 tests)
- **Lint**: `oxlint` + `lint:strict` (0 errors)
- **Docker**: Multi-stage build passes

**Green CI = certified numbers.** The data room and README cite only CI-verified values.

## Release Process

1. Local gates pass (lib 225 / vitest 356 / e2e 54 / lint:strict 0-0)
2. Tag created (`git tag v0.3.0`)
3. `release.yml` builds artifacts + runs `verify_release.ps1`
4. Small artifacts (SBOM, provenance, manifest) attached to GitHub release
5. Binaries ship via the website with SHA-256 verification

## Verification

```powershell
# Verify release integrity
./tools/verify_release.ps1 -Version "0.3.0"

# Verify facts are fresh
./tools/check_facts_fresh.ps1

# Verify no manual numbers
./tools/check_no_manual_numbers.ps1
```
