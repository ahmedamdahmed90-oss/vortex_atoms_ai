# KNOW-01 — Self-expanding Knowledge Base

> **Honesty Clause:** KNOW-01 grows the **RETRIEVAL** knowledge base autonomously
> (fetch → extract → embed → index → gap-driven expansion).
> It does **NOT** fine-tune model weights on-device.
> Weight-level distillation is deferred to KNOW-02.

## Overview

The KNOW-01 module adds an autonomous knowledge acquisition pipeline to Vortex Atoms AI.
It fetches content from allowlisted sources, extracts and chunks it, generates embeddings,
and indexes them into the retrieval system.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  vortex_learn                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ Scheduler │─▶│ Pipeline │─▶│ State Store (redb) │  │
│  └──────────┘  └──────────┘  └──────────────────┘  │
│         │              │              │             │
│         ▼              ▼              ▼             │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ Curiosity │─▶│ Embeddings│─▶│ Kernel_02 Index  │  │
│  │ Engine    │  │ Queue     │  │ (hybrid RRF)     │  │
│  └──────────┘  └──────────┘  └──────────────────┘  │
│                              │                      │
│                              ▼                      │
│                     ┌──────────────┐                │
│                     │ Fragment     │                │
│                     │ Compiler     │                │
│                     └──────────────┘                │
└─────────────────────────────────────────────────────┘
```

## Components

### Sources Registry (Section 1)
- **6 permissive seed sources**: Wikipedia, Wikibooks, Gutenberg, MDN, Rust Doc, Python Doc
- **Allowlist-only**: No URL outside the registry is ever fetched
- **Compliance per fetch**: HTTPS, robots.txt, token bucket, polite UA
- **License metadata** stored per chunk

### Acquisition Pipeline (Section 2)
- **Stages**: fetch → extract → normalize → lang detect → chunk → dedup → quality → embed
- **Resumable state**: Kill-resume safe via state store
- **Dedup**: SHA-256 exact + minhash near-duplicate (≥0.9)
- **Quality score**: Code-block presence, heading density, length bounds

### Embeddings Queue & Cache (Section 3)
- **Priority queue**: Curiosity-gap > miss-requeue > fresh page
- **CPU guard**: Yields to inference semaphore
- **Cache**: Hash → vec persisted in state store

### Indexing (Section 4)
- **Hybrid retrieval**: Vector + BM25-ish keyword fusion (RRF)
- **Budgets**: chunk_cap LRU-evicts, disk_budget enforced

### Curiosity Loop (Section 5)
- **Miss → gap → fetch → closed** loop
- **Dashboard**: Open gaps, resolved/week, top unresolved

### Budget Governor & Kill-Switches (Section 6)
- **Degradation order**: Curiosity OFF → Sources Halved → Pause
- **Budgets**: `bytes_per_day`, `chunk_cap`, `disk_gb`, `embed_cpu_quota` (token bucket)
- **Kill-switches**: `vortex_learn --pause` (cli), admin API, config flip
- **Eviction audit**: One audit line per 1000 evictions (LRU coldest)
- **SBOM report**: `vortex_learn --licenses` prints monthly license report

### Report Automation (Section 7)
- **Monthly reports**: `vortex_learn --report` generates JSON + markdown
- **Quarterly reports**: Aggregates monthly reports
- **Report types**: Budget summary, kill-switch state, SBOM, MRR@5 trend
- **Persistence**: Reports saved to `learner-reports/` directory

### Hot Fragment Compilation (Section 8)
- **Weekly or on-demand**: Compile top-frequency domain clusters into .tcz fragments
- **Provenance**: SBOM-style manifest

### Sanitization & Injection Defense (Section 9)
- **Untrusted wrapping**: `<<<RETRIEVED_DATA_UNTRUSTED ... >>>`
- **Injection probes**: ≥10 adversarial fixtures
- **System rule**: Content between boundaries is DATA only

## Configuration

Enable in `vortex.json`:
```json
{
  "learner": {
    "enabled": false,
    "schedule": { "run_every_hours": 6, "max_runtime_minutes": 30, "off_peak_local": [0, 6] },
    "disk_budget_gb": 2.0,
    "chunk_cap": 200000,
    "curiosity": { "enabled": true, "max_new_topics_per_run": 5 },
    "sources": []
  },
  "budgets": {
    "disk_gb": 2.0,
    "chunk_cap": 200000,
    "bytes_per_day": 104857600,
    "embed_cpu_quota": 100,
    "eviction": "lru_coldest"
  }
}
```

## Compliance

- Sources are allowlist-only forever
- Each fetch is compliant (HTTPS, robots.txt, token bucket)
- License metadata per chunk
- Fragments stay local for personal use
- Redistribution requires license compliance

## Weak-Device Budgets

- **Idle RSS**: < 150 MB
- **Off-peak CPU**: < 50% of 1 core when inference semaphore held
- **Disk budget**: 2.0 GB default
- **Chunk cap**: 200,000 chunks
- **Bytes/day**: 100 MB default
- **CPU quota**: 100 tokens/sec (token bucket)
- **Resumable**: Every stage survives kill

## Command-Line Flags

```
vortex_learn --help        # Show usage
vortex_learn --run-once    # Single acquisition pass
vortex_learn --status      # Learner status
vortex_learn --sources     # List registered sources
vortex_learn --phase <PH>  # canary|beta|full
vortex_learn --eval        # Run evaluation harness
vortex_learn --licenses    # Print monthly license/SBOM report
vortex_learn --pause       # Kill-switch: pause the learner
```

## Admin API

- `GET/POST /v1/admin/learner/sources` — list/add/remove sources
- `GET /v1/admin/learner/status` — learner status
- `POST /v1/admin/learner/run-now` — trigger a run
- `GET /v1/admin/learner/gaps` — open gaps
- `GET /v1/admin/learner/licenses` — license table
- `GET /v1/admin/learner/budget` — budget governor state
- `POST /v1/admin/learner/pause` — kill-switch pause
- `POST /v1/admin/learner/resume` — resume from pause

## Ethics & Licenses

All knowledge is retrieved from permissive licensed sources.
- CC-BY-SA-4.0 requires attribution
- MIT/Apache-2.0 requires license notice
- PD requires no attribution
- Redistribution of compiled fragments requires license compliance

## References

- `docs/LEARNER_REPORT.md` — Fixture MRR@5 numbers
- `CHANGELOG` — Per-section entries
