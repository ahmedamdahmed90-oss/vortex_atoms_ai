# Code Hygiene Backlog

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.
> CODE-HYGIENE-01 → CODE-HYGIENE-02: Close oxlint warnings, reach 0.

## Summary
- **Total warnings**: 94 (measured 2026-09-18)
- **Fixed**: 94 warnings across 4 batches + CODE-HYGIENE-02
- **Remaining**: 0 warnings
- **Errors**: 0

## Resolution Summary

### CODE-HYGIENE-01 — Batch 1 (~72 warnings)
Removed unused imports across 19 files. Zero behavior change.

### CODE-HYGIENE-01 — Batch 2 (~22 warnings)
Prefixed unused variables/parameters with `_` or removed from destructuring. Zero behavior change.

### CODE-HYGIENE-01 — Batch 3 (~6 warnings)
Added `// eslint-disable-next-line` suppressions. Changed `&&` side-effect patterns to `if` statements. Zero behavior change.

### CODE-HYGIENE-02 — Batch 4 (3 deferred → CLOSED)
- **useAccessibility.ts:94** — `exhaustive-deps` on `onSelect` → **CLOSED (latest-ref pattern)**. Replaced `useCallback` deps with `[]`, all values read via refs (`onSelectRef`, `focusedIndexRef`, etc.). Added test: "escape handler invokes the LATEST onSelect after parent re-render".
- **useWebSocket.ts:28** — `exhaustive-deps` on `setStreaming` → **CLOSED (dep-prune)**. Added `setStreaming` to dependency array (stable Zustand action, no re-run). Added test: "streaming state transitions through the socket handler".
- **ChatInput.tsx:21** — `exhaustive-deps` on `currentMessage` → **CLOSED (dep-prune)**. Removed unused `currentMessage` from `adjustHeight` useCallback deps and useEffect deps.

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
