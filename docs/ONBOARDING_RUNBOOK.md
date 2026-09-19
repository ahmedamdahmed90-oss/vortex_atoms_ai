# KNOW-01 Pilot Onboarding Runbook

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.

---

## Step 1: Install

```
.\VortexAtomsAI-Setup.exe
```

Verify: `curl http://127.0.0.1:8080/v1/health` returns 200 OK.

## Step 2: Tokens Rotation

```bash
curl -X POST http://127.0.0.1:8080/v1/admin/tokens/rotate \
  -H "Authorization: Bearer <admin_token>"
```

Verify: new token returned, old token invalidated.

## Step 3: Learner Canary Caps for Pilots

```bash
vortex_learn --phase canary --run-once
```

Max 50 pages/day for pilot users. Verify in telemetry output.

## Step 4: Support Channel

- **Email:** vortexatoms@gmail.com
- **SLA:** 48-hour reply on all pilot communications
- **Weekly office hours:** Scheduled via Calendly link
- **No telemetry without written consent**

## Step 5: Escalation Path

1. **Support:** vortexatoms@gmail.com (48h SLA)
2. **Engineering:** Direct developer contact via pilot Slack channel
3. **Escalation:** Ahmad Mansour (ahmed.mansour@vortex.atoms)
4. **Rollback:** Config flip / revert tag — never surgery on shipped bits

## Step 6: Verification Gate

After onboarding:
- `cargo test --features learner --lib`: 155 passed, 0 failed
- `cargo clippy --all-targets --features learner -- -D warnings`: 0 errors
- `vortex_learn --licenses`: SBOM report generated
- `vortex_learn --eval`: MRR@5 table from LEARNER_REPORT
- `vortex.json`: learner.enabled = false (shipped config)

## Step 7: Feedback Loop

- Weekly survey (5 questions)
- Submit to docs/MARKET_REPORT.md
- Requested topics enter learner gaps with source="pilot"
- Curiosity loop honors manual gaps
