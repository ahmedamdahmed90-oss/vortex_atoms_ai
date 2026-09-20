# KNOW-01 Outreach Kit

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.
> Honesty clause: every claim maps to a README / LEARNER_REPORT number.

---

## 1. Investor Email (EN, 120 words max)

**Subject:** Vortex Atoms AI v0.2.0 — local-first AI, compliance by design

Dear [Name],

Vortex Atoms AI ships a local-first AI assistant with compliance built in — not bolted on. All data stays on the user's machine; nothing is sent to third-party servers. The v0.2.0 release is certified: 155 Rust tests pass with 0 errors, 354 frontend tests green, 54 E2E tests stable. The learner module grows retrieval knowledge through compliant retrieval only — no weight training, no model fine-tuning.

Our differentiation is the honesty clause: every marketing claim is traceable to a README number. We serve users who value privacy over cloud convenience.

**Ask:** 20-minute call to discuss the pilot program and early access terms.

---

## 2. Pilot Customer Email (EN)

**Subject:** Pilot program — 90 days free, local-first AI

What we give:
- 90 days free access to all features
- Setup help and weekly office hours
- Direct access to our engineering team

What we ask:
- Weekly feedback (short survey)
- Optional anonymized telemetry with written consent
- One case-study quote (optional)

All data stays on your machine. No telemetry without your written consent.

---

## 3. Community Post — r/rust

**Title:** [PROJECT] Vortex Atoms AI — local-first AI with compliance by design

We built a local-first AI assistant in Rust with zero network dependencies for model loading. The 5-Kernel Matrix runtime achieves <50ms fastpath inference on commodity hardware. All retrieval growth uses compliant, allowlist-only sources with attribution tracking — the learner module (155 tests green) grows knowledge without any weight training.

Links: [README](README.md) · [LEARNER_REPORT](docs/LEARNER_REPORT.md) · [Demo Runbook](docs/DEMO_RUNBOOK.md)

Respect the subreddit rules — technical depth first, links last.

---

## 4. LinkedIn 4-Post Series

### Post 1 (Week 1, Day 1)
**Hook:** "I built an AI assistant on a laptop from 2011."
**Body:** Vortex Atoms AI runs entirely locally — no cloud calls, no data leaks. 155 tests green. The engine is the 5-Kernel Matrix: fastpath <50ms on commodity hardware.
**CTA:** "Local-first is not a limitation — it's a promise."
**Hashtags:** #LocalAI #PrivacyFirst #RustLang

### Post 2 (Week 1, Day 4)
**Hook:** "What if your AI ran in <50ms without a GPU?"
**Body:** The fastpath engine delivers <50ms latency on an 8GB laptop. No CUDA needed. No cloud needed. The learner module grows knowledge through compliant retrieval — not weight training.
**CTA:** "Speed and privacy are not trade-offs."
**Hashtags:** #Fastpath #EdgeAI #ComplianceByDesign

### Post 3 (Week 2, Day 1)
**Hook:** "We ship AI with an honesty clause."
**Body:** Every marketing claim maps to a README number. 155 tests green, 0 clippy errors, 99.49% frontend coverage. The learner module uses allowlist-only sources with license tracking and attribution requirements.
**CTA:** "Trust is the only moat worth claiming."
**Hashtags:** #OpenSource #HonestyClause #Trust

### Post 4 (Week 2, Day 4)
**Hook:** "90-day pilot — local-first AI, zero risk."
**Body:** We're offering 90 days free for pilot customers. Setup help, weekly office hours, and direct engineering access. All data stays on your machine. No telemetry without written consent.
**CTA:** "Reply to this post or email vortexatoms@gmail.com."
**Hashtags:** #PilotProgram #LocalFirst #AI

---

## 5. X/Twitter 8-Tweet Thread

**Thread: Local-first AI that ships honest**

1/ I built an AI assistant that runs entirely on your laptop. No cloud. No data leaks. No GPU required. < 50ms fastpath inference. Thread 🧵

2/ The 5-Kernel Matrix runtime delivers <50ms latency on commodity hardware. No CUDA. No cloud calls for model loading. Everything is local-first by design.

3/ Our learner module (155 tests green) grows retrieval knowledge through compliant retrieval only. No weight training. No fine-tuning. Just honest knowledge acquisition.

4/ We ship with an honesty clause: every marketing claim maps to a README number. 155 tests green, 0 clippy errors, 354 frontend tests, 54 E2E tests.

5/ The learner module uses allowlist-only sources. Every source has a license recorded. Attribution is tracked per chunk. Compliance is not optional — it's the design.

6/ Privacy is not a feature — it's the architecture. All data stays on your machine. No telemetry without written consent. The pilot program is 90 days free.

7/ Our certified numbers: Rust 155/155 tests, clippy 0 errors, frontend 99.49% coverage, E2E 54/54. Every number is traceable to README / LEARNER_REPORT.

8/ Join the 90-day pilot. Setup help, weekly office hours, direct engineering access. Reply below or email vortexatoms@gmail.com. Local-first AI, zero risk.

---

## 6. Positioning Guardrail Test

**Banned phrases:**
- أسرع من / faster than ChatGPT
- unlimited / بدون حدود سرعة
- fastest / الأسرع

**Check command:** `tools/check_claims.ps1`

This script greps all outreach docs for banned phrases. Gate red if found.

---

## 7. Email Signature Block

Use this signature in all outreach emails:

```
Ahmad Mansour
Founder, Vortex Atoms AI
https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai
vortexatoms@gmail.com
```

## 8. Website / Badge Reference

Badge URL for embedding:
```
https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/actions/workflows/ci.yml/badge.svg
```
