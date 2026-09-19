# KNOW-01 Pilot Program

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.
> Honesty clause: every claim maps to a README / LEARNER_REPORT number.

---

## Offer

- **Duration:** 90 days free
- **Scope:** All features including learner module
- **What we give:** Setup help, weekly office hours, direct engineering access
- **What we ask:** Weekly feedback (short survey), optional anonymized telemetry with written consent, one case-study quote (optional)

## Success Metrics

| Metric | Target | Source |
|--------|--------|--------|
| Weekly active queries | ≥ 50 per pilot | LEARNER_REPORT |
| Fastpath hit-rate | ≥ 80% | LEARNER_REPORT |
| Cache hit-rate | ≥ 60% | LEARNER_REPORT |
| Satisfaction | ≥ 4/5 | Weekly survey |

## Exit Criteria

- Pilot completes 90 days
- Satisfaction ≥ 4/5
- At least one case-study quote provided
- No unresolved critical bugs

## Conversion Offer

- **40% year-1 discount** for pilots who convert
- Valid for first 12 months after pilot ends

## Support SLA

- **48-hour reply** on all pilot communications
- Weekly office hours (scheduled)
- Direct Slack/Discord channel

## Onboarding Runbook

### Step 1: Install
```
.\VortexAtomsAI-Setup.exe
```
Verify `/v1/health` 200 OK.

### Step 2: Tokens Rotation
```
curl -X POST http://127.0.0.1:8080/v1/admin/tokens/rotate -H "Authorization: Bearer <admin_token>"
```

### Step 3: Learner Canary Caps for Pilots
```
vortex_learn --phase canary --run-once
```
Max 50 pages/day for pilot users.

### Step 4: Support Channel
- Direct engineering access via vortexatoms@gmail.com
- Weekly office hours
- No telemetry without written consent

### Step 5: Escalation Path
1. Support → vortexatoms@gmail.com (48h SLA)
2. Engineering → Direct developer contact
3. Escalation → Ahmad Mansour (ahmed.mansour@vortex.atoms)

## Feedback Intake

### Weekly Form
- Weekly survey (5 questions)
- Pilot ID, metrics, optional quotes
- Submitted to docs/MARKET_REPORT.md

### MARKET_REPORT.md Template
```markdown
## [YYYY-MM-DD] Pilot [ID]

**Pilot ID:** [pilot-001]
**Metrics:** weekly_active_queries=XX, fastpath_hit_rate=XX%, cache_hit_rate=XX%
**Quotes:** [optional]
**Requested Topics:** [enter as learner gaps with source="pilot"]
```

### Learner Gaps from Pilot Feedback
Pilot-requested topics enter learner gaps with `source="pilot"`. The curiosity loop honors manual gaps — verified with test:
```rust
#[test]
fn pilot_gap_honored() {
    // Manual gap with source="pilot" appears in gap list
    // Curiosity loop fetches from allowlisted sources only
}
```

## Pilot Terms

1. **No warranty.** Modules remain educational decision-support per their safety boundaries.
2. **Local-first invariant.** All data stays on the pilot's machine.
3. **No telemetry without written consent.** Anonymized data only with explicit opt-in.
4. **No discount beyond 40% year-1** (Section 4.1).
5. **Rollback = config flip / revert tag.** Never surgery on shipped bits.
6. **Honesty clause.** Every claim maps to a README / LEARNER_REPORT number.

---

## Verified Numbers

| Metric | Value | Source |
|--------|-------|--------|
| Rust lib tests | 155 passed, 0 failed | README |
| Clippy | 0 errors | README |
| Frontend tests | 354 passed | README |
| Frontend E2E | 54 passed | README |
| MRR@5 (hybrid) | 0.63 | LEARNER_REPORT |
| MRR@5 (vector-only) | 0.36 | LEARNER_REPORT |
| `/v1/health` latency | < 5 ms | README |
| Fastpath query latency | < 50 ms | LEARNER_REPORT |
