# Code Hygiene Backlog

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.
> CODE-HYGIENE-01 → CODE-HYGIENE-02: Close oxlint warnings, reach 0.

## Summary
- **Total warnings**: 94 (measured 2026-09-18)
- **Fixed**: 94 warnings (0 remaining)
- **Errors**: 0

## Resolution Summary

### CODE-HYGIENE-01 — Batches 1-3 (~100 warnings)
- Batch 1: Removed unused imports across 19 files
- Batch 2: Prefixed unused variables/parameters with `_` or removed
- Batch 3: Added suppressions, changed `&&` to `if` statements

### CODE-HYGIENE-02 — Deferred CLOSED (3 warnings)
- `useAccessibility.ts:94` — **latest-ref pattern** (useCallback deps `[]`, refs for all values)
- `useWebSocket.ts:28` — **dep-prune** (added `setStreaming` to deps, stable Zustand action)
- `ChatInput.tsx:21` — **dep-prune** (removed unused `currentMessage` from deps)

## Patterns Used
- **latest-ref**: Refs for unstable callbacks; `useCallback` deps stay `[]`
- **dep-prune**: Add stable functions to deps; remove unused deps
- **suppress**: `// eslint-disable-next-line` for intentional patterns

## Gates
- `npm run test`: 356 tests passed (22 files)
- `npx oxlint`: 0 warnings, 0 errors
- `npm run build`: ✓ built
- `npm run lint:strict`: `oxlint --deny warnings` — promoted to gate status

## Resolution Policy
- Warnings are fixed in batches with zero behavior change
- Any new warning introduced must be resolved before merge
- Design-change warnings are documented as deferred, never forced
