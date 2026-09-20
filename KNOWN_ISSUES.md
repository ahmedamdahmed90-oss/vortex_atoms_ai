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

These flakes are Firefox-specific timing issues, not logic bugs. They pass on retry and do not affect Chromium or WebKit runs. See individual issues for details.
