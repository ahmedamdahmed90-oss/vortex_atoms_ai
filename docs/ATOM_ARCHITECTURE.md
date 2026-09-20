// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# ATOM ARCHITECTURE — Vortex Atoms AI

> Stage 6–7 design record. Status legend: **Implemented** / **Partial** /
> **Planned**. Only Implemented items are backed by code + tests.

## What an Atom is

An Atom is the unit of loadable runtime capability: identity + kind +
lifecycle state + budget estimate. It is a vocabulary over machinery that
already existed — the knowledge orchestrator's registry → load → touch →
evict cycle — not a rewrite of it.

```rust
AtomDescriptor { id, kind, estimated_bytes, state }
AtomKind  ::= KnowledgeFragment | FastpathModule | InferenceSession
AtomState ::= Registered | Inactive | Loading | Active
            | Suspended | Evicted | Failed
```

## State machine (Implemented, tested in `src/atom.rs`)

Legal transitions only (everything else rejected, state unchanged):

- Registered → Loading | Inactive
- Inactive → Loading
- Loading → Active | Failed
- Active → Suspended | Evicted | Failed
- Suspended → Active | Evicted
- Evicted → Loading (resurrection path)
- Failed → Registered (explicit re-register; failure is sticky)

`is_resident()` = Active | Suspended.

## Resource budget (Implemented)

`ResourceBudget::default()` centralizes previously hard-coded constants
(values unchanged):

| Field | Value | Was |
|-------|-------|-----|
| `max_memory_bytes` | 256 MiB | matrix config + supervisor default literals |
| `max_active_atoms` | 8 | orchestrator `max_loaded_fragments` default |
| `max_concurrent_inference` | 2 | `MAX_CONCURRENT_INFERENCE` semaphore |
| `max_context_tokens` | 4096 | static upper bound (per-tier limits still apply) |

`FiveKernelMatrixConfig::default` and `SupervisorState::default` now
reference the budget instead of repeating literals.

## Metrics (Implemented)

`AtomMetrics` wired into `KnowledgeOrchestratorState`:

| Counter | Bumped on |
|---------|-----------|
| `atom_load_count` | `insert_loaded` |
| `atom_eviction_count` | `remove_loaded` (any path: capacity, age, intent) |
| `atom_cache_hits` / `misses` | `touch_loaded_bytes` hit / miss |
| `hit_rate()` | derived |

Observers (pure, no mutation): `atom_state_of()` (Active when resident,
Evicted when registered-but-unloaded), `active_atom_count()`,
`resident_bytes()` (loaded compressed bytes — an estimate, not RSS).

## Migration phases (atom policy A–G)

| Phase | Status | Evidence |
|-------|--------|----------|
| A — Atom abstraction | **Implemented** | `src/atom.rs` + 5 state-machine tests |
| B — Knowledge Atom adapter | **Implemented** | observers + metrics on the orchestrator |
| C — Atom lifecycle | **Partial** | lifecycle vocabulary + metrics live; routing/eviction logic itself unchanged (it already worked) |
| D — Resource budget | **Implemented** | centralized struct, two defaults reference it |
| E — Routing integration | Planned | router still uses intent labels + cosine rank directly |
| F — Eviction/cache | **Partial** | existing LRU-ish eviction kept + now counted; no policy change |
| G — Benchmark | **Partial** | registry/load micro-benchmarks in BENCHMARKS.md; GB-scale RSS experiment designed, not executed |

## Honesty notes

- The Atom layer is currently **observability + vocabulary**, not a new
  execution engine. The orchestrator still does the work; atoms name it.
- `resident_bytes` counts loaded fragment bytes, not process RSS. Memory
  claims stay at that level until the GB-scale experiment (Stage 14) runs.
- Eviction victim choice ties (equal access counts + same-ms timestamps)
  resolve by HashMap order — any coldest victim is valid; tests pin
  strictly ordered counts so they stay deterministic.
