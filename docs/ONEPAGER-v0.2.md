# KNOW-01 One-Pager — VortexAtomsAI v0.2.0

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.
> **NOTE:** This document reflects v0.2.0 state (2026-09-20). Current certified numbers: 156 Rust / 356 Vitest / 54 E2E. See README.md for live state.
> Honesty clause: every claim maps to a README / LEARNER_REPORT number.

## Generation Instructions

1. Open `http://127.0.0.1:8080` in a browser
2. Press `Ctrl+P` (or `Cmd+P`)
3. Select **Save as PDF**
4. Verify page breaks in print CSS (`frontend/src/styles/print.css`)
5. Name: `VortexAtomsAI-OnePager-v0.2.pdf`

## Certified Numbers (as of v0.2.0)

| Metric | Value | Source |
|--------|-------|--------|
| Rust lib tests | 155 passed, 0 failed | README |
| Clippy | 0 errors | README |
| Frontend tests | 354 passed | README |
| Frontend E2E | 54 passed | README |
| Frontend coverage | 99.49% statements | README |
| MRR@5 (hybrid) | 0.63 | LEARNER_REPORT |
| MRR@5 (vector-only) | 0.36 | LEARNER_REPORT |
| MRR@5 delta | +0.27 | LEARNER_REPORT |
| `/v1/health` latency | < 5 ms | README |
| Fastpath query latency | < 50 ms | LEARNER_REPORT |

## Positioning

- **Not** weight training — compliant retrieval growth only
- Local-first: all data stays on the user's machine
- No speed overclaims on old hardware
- Arabic-first RTL interface
- Honesty clause: every claim is verified
