# KNOW-01 Demo Runbook

> **Copyright (c) 2026 Ahmad Mansour.** All rights reserved.
> Honesty clause: every claim below maps to a README / LEARNER_REPORT number.

Reference machine: Windows 11, 8 GB RAM, SSD, HDD model cache.

---

## demo-01-health

**Command:** `curl http://127.0.0.1:8080/v1/health`
**Expected output:** `{"status":"ok","uptime_ms":...}` (< 5 ms)
**Capture:** `assets/demo-01-health.png`

## demo-02-device

**Command:** `curl http://127.0.0.1:8080/v1/device`
**Expected output:** `{"tier":"eco","model_count":...}` (< 5 ms)
**Capture:** `assets/demo-02-device.png`

## demo-03-avian-query

**Command:** `curl -X POST http://127.0.0.1:8080/v1/generate -d '{"prompt":"avian genetics summary","max_tokens":50}'`
**Expected output:** response with `engine: "fastpath"`, latency < 50 ms
**Capture:** `assets/demo-03-avian-query.png`

## demo-04-avian-cache

**Command:** Same as demo-03 (repeat query)
**Expected output:** response with `engine: "cache"`, latency near-instant
**Capture:** `assets/demo-04-avian-cache.png`

## demo-05-learner-canary

**Command:** `vortex_learn --phase canary --run-once`
**Expected output:** live telemetry tail showing canary source (wikipedia_en only), `max_pages=25`, telemetry JSON on stderr
**Capture:** `assets/demo-05-learner-canary.mp4`

## demo-06-learner-eval

**Command:** `vortex_learn --eval`
**Expected output:** MRR@5 table from `docs/LEARNER_REPORT.md` (hybrid 0.63 vs vector 0.36, delta +0.27)
**Capture:** `assets/demo-06-learner-eval.png`

## demo-07-tray-icon

**Command:** Left-click tray icon
**Expected output:** default browser opens to `http://127.0.0.1:8080`
**Capture:** `assets/demo-07-tray-icon.png`

## demo-08-tier-switch

**Command:** `curl -X POST http://127.0.0.1:8080/v1/admin/device/tier -d '{"tier":"std"}'`
**Expected output:** `{"tier":"std","admin_audited":true}`
**Capture:** `assets/demo-08-tier-switch.png`

## demo-09-budget-governor

**Command:** `vortex_learn --phase canary --run-once` with fixture exceeding `disk_gb` budget
**Expected output:** degradation log: `Curiosity OFF → Sources Halved → Pause`, eviction audit line per 1000 evictions
**Capture:** `assets/demo-09-budget-governor.mp4`

---

## Capture Script Usage

```powershell
.\tools\capture_demo.ps1 -Step 1  # Captures demo-01-health.png
.\tools\capture_demo.ps1 -Step all # Runs all steps sequentially
```

Each step pauses for manual screen capture (no auto screen-recording; weak-machine friendly).

---

## Verified Numbers

| Metric | Value | Source |
|--------|-------|--------|
| `/v1/health` latency | < 5 ms | README certified state |
| `/v1/device` latency | < 5 ms | README certified state |
| Fastpath query latency | < 50 ms | LEARNER_REPORT |
| MRR@5 (hybrid) | 0.63 | LEARNER_REPORT |
| MRR@5 (vector-only) | 0.36 | LEARNER_REPORT |
| MRR@5 delta | +0.27 | LEARNER_REPORT |
| Rust lib tests | 156 passed, 0 failed | README |
| Clippy | 0 errors | README |
| Frontend tests | 356 passed | README |
| Frontend E2E | 54 passed | README |
