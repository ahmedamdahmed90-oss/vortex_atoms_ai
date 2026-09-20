# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# SYNC-02 Section 1: Docs Historical References Decisions

Generated: 2026-09-20

## Decision Table

| File | Type | Decision | Reason |
|------|------|----------|--------|
| docs/OUTREACH_KIT.md | Current | UPDATE | No date header, claims to be live outreach material |
| docs/MARKET_REPORT.md | Current | UPDATE | "auto-appended by tools/market_weekly.ps1" — living doc |
| docs/ONEPAGER-v0.2.md | Snapshot | ADD NOTE | Title explicitly says "v0.2.0" — dated snapshot |
| docs/PILOT_PROGRAM.md | Current | UPDATE | No date, "KNOW-01" prefix, used for active pilot outreach |
| docs/DEMO_RUNBOOK.md | Current | UPDATE | No date, "KNOW-01" prefix, used for live demos |
| docs/LEARNER_REPORT.md | Snapshot | ADD NOTE | "Baseline (KNOW-01 close)" — frozen metrics snapshot |

## Changes Made

### Updated (current docs)

- **OUTREACH_KIT.md**: 7 occurrences fixed (155→156, 354→356)
- **MARKET_REPORT.md**: 3 occurrences fixed (155→156, 354→356)
- **PILOT_PROGRAM.md**: 2 occurrences fixed (155→156, 354→356)
- **DEMO_RUNBOOK.md**: 2 occurrences fixed (155→156, 354→356)

### Notes added (snapshots)

- **ONEPAGER-v0.2.md**: Added dated note at top: "This document reflects v0.2.0 state (2026-09-20). Current certified numbers: 156 Rust / 356 Vitest / 54 E2E."
- **LEARNER_REPORT.md**: Added dated note at top: "This document reflects baseline state (2026-09-20). Current certified numbers: 156 Rust / 356 Vitest / 54 E2E."

## Section 2 — Data Room

| Check | Status | Evidence |
|-------|--------|----------|
| facts.json exists | ✅ PASS | Updated version to v0.3.1-sync02 |
| facts.json numbers correct | ✅ PASS | 156/356/54/99.49% already correct |
| build_data_room.ps1 | ⚠️ PARTIAL | `_render.py` missing — script fails at Python step |
| Data room output exists | ✅ PASS | 22 files in `docs/data_room/out/v0.3.0-dataroom/` |
| check_numbers.ps1 | ✅ PASS | GREEN: All numbers synchronized |
