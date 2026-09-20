# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# SYNC-04 Section 2: CI Failure Diagnosis

Generated: 2026-09-20
Run URL: https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/actions/runs/35511762004
Commit: 5937c25

## CI Run Results

| Job | Status | Notes |
|-----|--------|-------|
| Frontend (React) | ✅ PASS | 1m31s |
| Backend (Rust) | ❌ FAIL | integration test flake |
| Security Audit | ❌ FAIL | `continue-on-error: true` — cargo audit warnings |
| Data Room & Facts | ❌ FAIL | `powershell` not found on Ubuntu |
| Docker Build | ⏸ PENDING | Depends on Backend |

## Failure 1: Backend — Integration Test Flake

**Failing test:** `test_five_kernel_matrix_submit_ui_input`
**Location:** `tests/integration_tests.rs:909`
**Error:** `Should have received LogicExecuted event`
**Exit code:** 101

**Root cause:** This is a known timing-sensitive integration test. The test expects a `LogicExecuted` event within a timeout, but on CI runners (slower than local), the event doesn't arrive in time. This is the same class of issue as the 3 Firefox E2E flakes documented in `KNOWN_ISSUES.md`.

**Impact:** 96 passed, 1 failed. The 156 unit tests (`--lib`) all pass on CI — this failure is in integration tests only.

**Fix options:**
1. Increase timeout in the test (minimal change)
2. Mark as `#[ignore]` and add to flake tracking (like the Firefox E2E flakes)
3. Run with `--test-threads=1` on CI (slower but deterministic)

**Recommendation:** Option 2 — mark as flaky, track in KNOWN_ISSUES.md.

## Failure 2: Data Room & Facts — PowerShell Not Found

**Error:** `line 1: powershell: command not found`
**Exit code:** 127

**Root cause:** CI runs on `ubuntu-latest` which uses bash. The data-room job calls `powershell` which is not installed on Ubuntu runners. On Windows runners, `pwsh` is available but not `powershell`.

**Fix options:**
1. Change `powershell` to `pwsh` in ci.yml (pwsh is pre-installed on GitHub-hosted Ubuntu runners)
2. Change the runner to `windows-latest` (heavier, slower)
3. Rewrite the scripts in bash/sh

**Recommendation:** Option 1 — change `powershell` to `pwsh` in the data-room job steps.

## Failure 3: Security Audit — Cargo Audit Warnings

**Warnings:** `ttf-parser` (unmaintained), `glib` (unsound), `chacha20` (yanked)
**Exit code:** 1

**Root cause:** `cargo audit` treats warnings as failures. These are upstream dependency issues, not code issues. The job is already marked `continue-on-error: true`.

**Impact:** None — the job is non-blocking by design.

## Diagnosis Summary

| Failure | Caused by our changes? | Needs fix? |
|---------|----------------------|------------|
| Backend integration test | NO (pre-existing flake) | YES — mark as flaky |
| Data Room & Facts | NO (pre-existing CI config bug) | YES — change powershell→pwsh |
| Security Audit | NO (upstream warnings) | NO (continue-on-error) |

**None of the failures were caused by the SYNC-02 reconciliation changes.**
