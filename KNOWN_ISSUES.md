---
title: Known Issues
---

# KNOWN_ISSUES.md

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.

## E2E Test Flakes (Firefox)

| # | Symptom | Browser | Status | Issue |
|---|---------|---------|--------|-------|
| 1 | Intermittent timeout on sidebar toggle (mobile viewport) | Firefox | Retry-green | [#1](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/issues/1) |
| 2 | Race condition in theme toggle assertion | Firefox | Retry-green | [#2](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/issues/2) |
| 3 | Focus trap test flake on keyboard navigation | Firefox | Retry-green | [#3](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/issues/3) |

### Retry Policy

- **Max retries:** 1 (label: `retry-green`)
- **Flaky test label:** `flaky`
- **Target fix date:** 2026-10-15

### How to reproduce

```bash
cd frontend
npx playwright test --project=firefox --repeat-each=10
```

### Root cause notes

| # | Root cause |
|---|------------|
| 1 | `ChatSidebar` ignored `isOpen` on mobile — the fixed RTL `aside` always covered the toggle button, so clicks hit the conversations panel instead of `فتح الشريط الجانبي`. |
| 2 | Theme dropdown option clicked while `animate-in` was still opening; Firefox hit the backdrop overlay instead of the option. |
| 3 | `Tab` fired before Firefox committed focus after click, so `activeElement` was still BODY / pre-click element on first try. |

Fixes applied in `frontend/e2e/chat.spec.ts` (waits, `toBeFocused` + `expect.poll`) and `frontend/src/components/chat/ChatSidebar.tsx` (closed state: `translate-x-full` + `invisible`, restored on `lg`).

These flakes were Firefox-specific timing issues, not logic bugs. They pass on retry and do not affect Chromium or WebKit runs. See individual issues for details.
