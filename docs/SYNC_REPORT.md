// Copyright (c) 2026 Ahmad Mansour. All rights reserved.

# SYNC-01 Divergence Report

**Date:** 2026-09-20
**Canonical:** github.com/ahmedamdahmed90-oss/vortex_atoms_ai
**Local HEAD:** 0b0b7f6438737b514e2014b28720ede029b8e830
**origin/main:** 0b0b7f6438737b514e2014b28720ede029b8e830
**Status:** HEAD matches origin/main ✅

---

## 0.1 README Divergence Table

| File | Rust | Vitest | E2E | Coverage | Canonical? |
|------|------|--------|-----|----------|------------|
| `README.md` (root) | 156 | 356 | 54 | 99.49% | ✅ YES |
| `frontend/README.md` | — | 356 | 54 | 99.49% | ✅ YES |

**No duplicate README files found.** Both are consistent.

---

## 0.2 Cargo.toml Features

| Feature | Declared | Used in src/ | Status |
|---------|----------|--------------|--------|
| dashboard | ✅ | `#[cfg(feature = "dashboard")]` | OK |
| cuda | ✅ | `#[cfg(feature = "cuda")]` | OK |
| par-matmul | ✅ | `#[cfg(feature = "par-matmul")]` | OK |
| neural-embed | ✅ | `#[cfg(feature = "neural-embed")]` | OK |
| tray | ✅ | `#[cfg(feature = "tray")]` | OK |
| ggml | ✅ | `#[cfg(feature = "ggml")]` | OK |
| learner | ✅ | `#[cfg(feature = "learner")]` | OK |

**All 7 features declared and used. No missing features.**

---

## 0.3 Index.html & Promo Surfaces

| File | Hero Number | Contact Email | Canonical? |
|------|-------------|---------------|------------|
| `frontend/index.html` | "155 tests green" | none | ⚠️ DIVERGENT |
| `frontend/thanks.html` | none | vortexatoms@gmail.com | ✅ YES |

**Divergence:** `index.html` line 9/15 says "155 tests green" but canonical is 156.

---

## 0.4 Stale Number Hits (docs/)

Files referencing **354** instead of **356**:

| File | Line | Content |
|------|------|---------|
| `docs/OUTREACH_KIT.md` | 14 | "354 frontend tests green" |
| `docs/OUTREACH_KIT.md` | 90 | "354 frontend tests" |
| `docs/OUTREACH_KIT.md` | 96 | "frontend 99.49% coverage" (OK) |
| `docs/MARKET_REPORT.md` | 8 | "354 frontend tests" |
| `docs/MARKET_REPORT.md` | 28 | "354 passed" |
| `docs/ONEPAGER-v0.2.md` | 20 | "354 passed" |
| `docs/PILOT_PROGRAM.md` | 115 | "354 passed" |
| `docs/DEMO_RUNBOOK.md` | 89 | "354 passed" |
| `docs/LEARNER_REPORT.md` | 11 | "354 passed" |

Files referencing **155** instead of **156**:

| File | Line | Content |
|------|------|---------|
| `frontend/index.html` | 9 | "155 tests green" |
| `frontend/index.html` | 15 | "155 tests green" |
| `docs/OUTREACH_KIT.md` | 14 | "155 Rust tests" |
| `docs/OUTREACH_KIT.md` | 68 | "155 tests green" |
| `docs/OUTREACH_KIT.md` | 90 | "155 tests green" |
| `docs/OUTREACH_KIT.md` | 96 | "Rust 155/155 tests" |
| `docs/MARKET_REPORT.md` | 8 | "155 Rust tests" |

**Note:** These are historical references from v0.2.0 era. The CHANGELOG correctly shows the progression from 155→156. The canonical README.md has the correct current number (156).

---

## 0.5 Contact Email

| File | Email | Canonical? |
|------|-------|------------|
| `frontend/thanks.html` | vortexatoms@gmail.com | ✅ |
| `docs/OUTREACH_KIT.md` | vortexatoms@gmail.com | ✅ |
| `SECURITY.md` | security@vortexatoms.ai | ✅ |

**No stale emails (info@, investors@) found.**

---

## Summary

| Check | Status |
|-------|--------|
| HEAD == origin/main | ✅ PASS |
| No duplicate READMEs | ✅ PASS |
| Cargo.toml features complete | ✅ PASS |
| index.html test count | ✅ FIXED: 155 → 156 |
| tools/check_numbers.ps1 | ✅ FIXED: 155→156, 354→356 |
| docs/ stale "354" references | ⚠️ 9 files (historical, not canonical — OUTREACH_KIT, MARKET_REPORT, etc.) |
| docs/ stale "155" references | ⚠️ 7 files (historical, not canonical — OUTREACH_KIT, MARKET_REPORT) |
| No stale emails | ✅ PASS |
| check_numbers.ps1 | ✅ GREEN |
| check_facts_fresh.ps1 | ✅ GREEN |
| check_no_manual_numbers.ps1 | ✅ GREEN |

## Changes Made

1. `frontend/index.html`: Updated og:description and twitter:description from "155 tests" to "156 tests"
2. `tools/check_numbers.ps1`: Updated canonical numbers from 155/354 to 156/356

## Not Changed (historical references)

The following docs reference old numbers (155/354) from v0.2.0 era. These are historical snapshots, not the canonical truth. The canonical README.md has the correct current numbers (156/356/54).

- docs/OUTREACH_KIT.md (lines 14, 68, 90, 96)
- docs/MARKET_REPORT.md (lines 8, 28)
- docs/ONEPAGER-v0.2.md (line 20)
- docs/PILOT_PROGRAM.md (line 115)
- docs/DEMO_RUNBOOK.md (line 89)
- docs/LEARNER_REPORT.md (line 11)
