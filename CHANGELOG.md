# Changelog

// Copyright (c) 2026 Ahmad Mansour. All rights reserved.

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Per-session inference isolation (2026-09-21)
- New `inference_session` module: sessions own history + sampler; weights
  stay shared-serial (Qwen2 weights are not `Clone`, candle KV lives in
  the model — full engine clones are impossible without vendor surgery).
- `LlmInference::fork_session` + `generate_with_session` /
  `generate_streaming_with_session`: swap history+sampler in/out around
  the unchanged hot loop (restored even on error).
- `/ws`: one session per connection (own multi-turn memory + sticky
  temperature); `/generate` + `/chat`: ephemeral session per request
  (temperature no longer leaks into later requests; old `clear_history`
  band-aid superseded). Kernels/IKC keep the global engine (unchanged).
- `VortexSampler::fork(salt)`: fresh RNG per session (7 new tests:
  4 session isolation + 3 fork determinism/divergence/config).
- Residual sharing (harmless, documented): n-gram tables (exact
  verifier), hash-validated prefix-KV, global cancel flag.

### WS origin check port fix (2026-09-21, found live)
- `ws_origin_allowed` stripped the port from Host but not from Origin,
  403ing every same-origin WS handshake on a non-default port — WS
  streaming was broken for all default-config users (the 403 body
  misleadingly said "read-only mode"). Now mirrors the correct CORS
  twin (`origin_host`, full host:port compare); cross-port stays denied.
  3 new tests. Found while live-verifying per-session streaming.

### Post-v0.2.0 fixes (2026-09-21, live-verified on Sandy Bridge i5-2430M)
- `tokens_per_second` in `/v1/generate` + Kernel_03 IKC events now reports
  measured completion-tokens/wall-time instead of hardcoded `0.0`
  (new `llm_api::throughput_tps` helper + 3 unit tests; convention mirrors
  `generate_streaming`). Measured: ~0.008 tok/s for Qwen2.5-0.5B —
  prefill-dominated, see `docs/BENCHMARKS.md`.
- Default server port `3000` → `8080`, matching README, Dockerfile,
  installer shortcuts, frontend defaults, and `--help` text (the code
  default was the sole outlier). Locked by unit test.
- Measured inference numbers added to `docs/BENCHMARKS.md`, closing the
  Stage 13 honest-scope gap (release 2-token completion ≈ 265s on the
  test machine; debug build unbounded-slow, appears hung but is not).
- Release workflow: frontend is now built before binaries (rust-embed
  requires `frontend/dist/` at compile time); `cargo auditable` replaced
  with `cargo build`; changelog generation fixed for first tag.
- Build: repo no longer pins `-C target-cpu=native` (crashed rustc with
  SIGILL on virtualized CI runners; tied binaries to the build CPU).

### v0.2.0 (2026-09-20) — Autonomous Staged Engineering Protocol

17-stage autonomous engineering execution applied to the repository.
All work is additive — no public API removed, no kernel count, route,
or feature changed.

#### Correctness & Unicode
- `chunk_text` and `truncate` fixed to use char boundaries instead of
  byte slicing (eliminated UTF-8 panics)
- 6 new Unicode tests added

#### Import Hardening
- `ImportReport` struct + `import_directory_report` for partial-failure
  surfacing
- Symlink, file-size, and extension guards in `import_directory`
- No silent knowledge loss — every per-file failure is reported

#### Embedding & Retrieval
- `EmbeddingModel` renamed to `FastHashEmbedder` (honest naming)
- `VectorIndex` trait added as ANN seam over linear scan
- O(N·D) scaling measured and documented (460/2314/11526 µs at N=200/1000/5000)
- Retrieval benchmark added

#### RAG Trust Boundary
- `ContextBuilder` with UNTRUSTED delimiters and char-budget enforcement
- `build_rag_context` compat shim updated
- 5 new RAG trust-boundary tests

#### Atom Abstraction & Lifecycle
- New `src/atom.rs`: `AtomKind`, `AtomState`, `AtomDescriptor`,
  `ResourceBudget`, `AtomMetrics`
- Orchestrator wired with metrics observers and centralized 256MiB budget
- 8 new lifecycle tests

#### Inference & Concurrency
- Speculative decoding gated strictly on `sampler.is_greedy()` (argmax-exact only)
- `clear_history()` on generate/tool_call/batch endpoints (fixes cross-client
  history leak)
- 3 new sampler correctness tests

#### Security, API & 5-Kernel
- Security audit clean — no critical vulnerabilities
- Kernel contracts documented + `kernel_liveness()` / `all_kernels_alive()`
  probes
- API inventory verified, metrics real, doc notes added

#### Performance & Benchmarks
- 3 ignored micro-benchmarks + `docs/BENCHMARKS.md`
- No blind optimizations — all performance claims backed by measurements
- Fixed flaky `eviction_records_metrics` test

#### Docs Sync
- `docs/AGENT_INITIAL_ARCHITECTURE.md`, `ENGINEERING_BASELINE.md`
- `docs/ATOM_ARCHITECTURE.md`, `ENGINEERING_REVIEW.md`, `BENCHMARKS.md`
- `docs/API_REFERENCE.md` statelessness notes added
- `docs/FINAL_ENGINEERING_REPORT.md` comprehensive before/after metrics

#### CI Fixes
- Fixed linker OOM on `learner,tray` tests: `CARGO_BUILD_JOBS=1` +
  `RUSTFLAGS="-C split-debuginfo=unpacked"`
- Updated `Cargo.lock`: h2 0.4.15→0.4.19, hyper 1.10.1→1.11.1
  (fixes RUSTSEC-2026-0194)
- Added disk cleanup step to prevent runner exhaustion
- All 4 CI jobs green: Backend (Rust), Frontend, Data Room, Security Audit

### CODE-HYGIENE-02 (2026-09-18)
- Closed last 3 deferred oxlint warnings (reached 0 warnings, 0 errors)
- **latest-ref pattern**: `useAccessibility.ts` — `useCallback` deps `[]`, all values read via refs
- **dep-prune**: `useWebSocket.ts` — added `setStreaming` to deps (stable Zustand action); `ChatInput.tsx` — removed unused `currentMessage` dep
- Added 2 unit tests: "escape handler invokes the LATEST onSelect after parent re-render" + "streaming state transitions through the socket handler"
- **lint:strict** promoted to gate status: `oxlint --deny warnings`
- Total: 94 warnings fixed, 0 remaining

### FUND-01 (2026-09-18) — Investor Data Room
- Created `docs/data_room/facts.json` — single source of truth for all investor-facing numbers
- Created `docs/data_room/out/v0.3.0-dataroom/` — 10 documents + SHA-256 manifest
- Created `tools/build_data_room.ps1` — generator tool (idempotent)
- Created `tools/check_facts_fresh.ps1` — freshness checker (≤30 days)
- Created `tools/check_no_manual_numbers.ps1` — guard against untraceable digits
- Created `docs/data_room/deck_outline.md` — 10-slide pitch deck with speaker notes (AR+EN)
- Created `docs/data_room/FAQ_HARD_QUESTIONS.md` — 12 hard Q/As with honest answers
- Created `docs/data_room/00_INDEX.md` — TOC, access log, NDA-lite note, checksum manifest
- Created `docs/data_room/03_SECURITY_POSTURE.md` — controls table + residual risks verbatim
- Created `docs/data_room/06_MARKET_TRACTION.md` — pilot slots OPEN, outreach log
- Created `docs/data_room/07_RELEASE_INTEGRITY.md` — SBOM/provenance/checksums
- Created `docs/data_room/08_FOUNDER_AND_ROADMAP.md` — self-taught founder, 18-month roadmap
- Created `docs/data_room/09_ASK_AND_USE_OF_FUNDS.md` — $500K-$1M, 60/25/15
- Created `docs/data_room/10_RISKS_AND_MITIGATIONS.md` — risk matrix with mitigation + owner
- README "Second wave" updated with per-item ✅/Deferred states
- CHANGELOG: SEC-01 + FUND-01 entries
- `cargo test --features learner --lib`: 156 passed, 0 failed
- `cargo clippy --all-targets --features learner,tray -- -D warnings`: 0 errors
- Frontend `npm run test`: 356 passed, 22 files

### SEC-01 (2026-09-18) — Security Master Prompt
- **Section 1 (Encryption at Rest)**: Added `at_rest` health flag, 5 integration tests in `tests/integration_tests.rs`
- **Section 2 (Transport Hijacking)**: WS origin check in `auth_middleware`, `security_headers_middleware` with CSP/X-Frame-Options/No-store/HSTS
- **Section 3 (Kiosk Abuse)**: `--read-only` flag blocks mutating admin paths, `SecurityTab` buttons disabled
- **Section 4 (Multi-Instance Conflicts)**: `SingleInstance` mutex, exit code 2, `tray_flash` notification
- **Section 5 (Disk Exhaustion)**: `check_disk_space` guard (1024 MB default), `min_disk_free_mb` config option
- **Section 6 (Request Flooding)**: Per-route body caps via `RequestBodyLimitLayer` (1MB/2MB/50MB)
- **Section 7 (Config Tampering)**: `replace_cfg` validation rejects unsafe relaxation, 422 + audit
- **Section 8 (Metrics Leak)**: `/v1/metrics` Prometheus text + `/v1/admin/metrics` JSON
- **Section 9 (Session Pile-up)**: `POST /v1/admin/sessions/purge` with confirm dialog, startup retention purge
- **Section 10 (Supply Chain)**: `SBOM.json` (715 components), `PROVENANCE.md`, `provenance.json`, SHA-256 sidecars, `tools/verify_release.ps1`
- `cargo test --features learner --lib`: 156 passed, 0 failed
- `cargo clippy --all-targets --features learner,tray -- -D warnings`: 0 errors
- Frontend `npm run test`: 356 passed, 22 files

### CODE-HYGIENE-01 (2026-09-18)
- Closed 91 of 94 oxlint warnings across 4 batches
- Batch 1: Removed unused imports across 19 frontend files (~72 warnings)
- Batch 2: Prefixed unused variables/parameters with `_` (~22 warnings)
- Batch 3: Added suppressions, changed `&&` side-effects to `if` statements (~6 warnings)

## [0.2.0] — 2026-09-18 (Learner Release)

### Added
- KNOW-02 Section 5: Budget Governor & Kill-Switches (`src/learner/governor.rs`)
- KNOW-02 Section 6: Report Automation (`src/learner/report.rs`)
- `vortex_learn --licenses`, `--pause`, `--resume`, `--report` flags
- Automated monthly/quarterly report generation (JSON + markdown)
- Degradation order: Curiosity OFF → Sources Halved → Pause
- All `SchedulerConfig`, `CuriosityConfig`, `SourceEntry`, `LicenseEntry`, `LearnerMetrics`, `EvalMetrics` fields now `#[serde(default)]` for graceful JSON parsing
- `cargo test --features learner --lib`: **155 passed, 0 failed**
- `cargo clippy --all-targets --features learner -- -D warnings`: **0 errors**

### Changed
- Version bump 0.1.0 → 0.2.0

### Deferred UI Ideas (noted here, no new dashboard work)
- Pilot dashboard tab (track weekly active queries, fastpath hit-rate)
- Pilot satisfaction score widget
- Pilot conversion offer banner
- These are tracked in MARKET_REPORT.md under [Deferred UI Ideas]

## [0.1.0]

### KNOW-02 — Staged Activation & Telemetry
- Section 0 complete: baseline frozen, vortex.json SHA-256 recorded
- `cargo test` baseline: 121/121 (no learner)
- `cargo test --features learner --lib`: 137 passed (up from 120)
- Frontend baseline: 22 files, 354 tests passed
- oxlint: 0 errors, 36 warnings logged to docs/CODE_HYGIENE.md
- docs/CODE_HYGIENE.md created (oxlint backlog)
- docs/LEARNER_REPORT.md baseline section added

### KNOW-02 Section 1 — Staged Live Activation
- Added `src/learner/canary.rs`: CanaryConfig, Watchdog, RunTelemetry, check_allowlist_miss
- Added `--phase canary` flag to `vortex_learn` binary
- Canary mode: single source (wikipedia_en), max_pages=25, max_runtime=5min, abort-on-allowlist-miss
- Watchdog: auto-pause on embed_queue_depth > 500 OR learner CPU > 60% for > 10 min while inference held
- 12 new canary tests added (watchdog, canary isolation, allowlist enforcement)
- `cargo test --features learner --lib`: 103 passed, 0 failed (up from 91)
- All clippy warnings resolved in canary.rs and vortex_learn.rs
- CHANGELOG entry

### KNOW-02 Section 2 — Telemetry & Run Logs
- Added `src/learner/telemetry.rs`: PersistentRunLog, RunLogBuffer, RunLogEntry, LearnerMetrics, LearnerRunsResponse, TelemetryAccumulator
- `learner_runs.jsonl` persistent append-only log with 50 MiB bounded ring buffer
- GET `/v1/admin/learner/metrics` + `/v1/admin/learner/runs` endpoints added to `LearnerApiState`
- `register_run_log()` for persisting run data
- Counter accuracy tests + JSONL resume tests (≥8 entries)
- `cargo test --features learner --lib`: 113 passed (up from 103)
- All clippy warnings resolved
- CHANGELOG entries per section

### KNOW-02 Section 3 — Evaluation Harness
- Added `src/learner/eval.rs`: EvalMetrics, EvalRun, RegressionResult, RetrievalResult, GoldenQuery
- `vortex_learn --eval` flag computes MRR@5 + hit@1 (hybrid RRF ablation)
- Regression rule: MRR@5 drop > 5% → health flag + dashboard banner + audit
- KNOW-01 injection probes run inside --eval; any leak = eval FAIL
- Golden retrieval set: 30 queries (20 English + 10 Arabic) in `tests/golden/retrieval/mod.rs`
- Tests: eval determinism, regression banner, probes green, golden query validation
- `cargo test --features learner --lib`: 120 passed (up from 113)
- CHANGELOG entries per section

### KNOW-02 Section 4 — Arabic & Multilingual Growth
- Added `src/learner/arabic.rs`: normalize_arabic, ArabicStemmer, LanguageWeights, FusionWeights, language_aware_fusion, arabic_seed_sources
- Added `ar_wikipedia` and `ar_wikibooks` to seed sources registry (CC-BY-SA-4.0)
- AR normalization: strip diacritics/tatweel, normalize alef/ya/ta-marbuta
- Light Arabic stemmer for keyword side of RRF
- Language-aware fusion weights: AR `keyword_boost=1.5`, EN `keyword_boost=1.0`
- Tests: normalizer ≥12 cases, stemmer ≥8, AR weights, Arabic seed sources
- `cargo test --features learner --lib`: 137 passed (up from 120)
- CHANGELOG entries per section

### KNOW-02 Section 5 — Budget Governor & Kill-Switches
- Added `src/learner/governor.rs`: BudgetGovernor, BudgetConfig, DegradeLevel, KillSwitchState, TokenBucket, EvictionAudit, SbomReport
- Degradation order: Curiosity OFF → Sources Halved → Pause
- Budgets: `bytes_per_day`, `chunk_cap`, `disk_gb`, `embed_cpu_quota` (token bucket)
- Kill-switches: `vortex_learn --pause`/`--resume`, admin API, config flip
- Eviction audit: one audit line per 1000 evictions (LRU coldest)
- SBOM report: `vortex_learn --licenses` prints monthly license/SBOM
- `SchedulerConfig` fields now `#[serde(default)]` for graceful JSON parsing
- `CuriosityConfig` fields now `#[serde(default)]` for graceful JSON parsing
- `LicenseEntry` moved to `api.rs` with `chunk_count` field
- `cargo test --features learner --lib`: **155 passed, 0 failed** (up from 137)
- `cargo clippy --features learner -- -D warnings`: **0 errors**
- CHANGELOG entries per section

### KNOW-02 Section 6 — Report Automation
- Added `src/learner/report.rs`: ReportGenerator, AutomatedReport, BudgetSummary, ReportPeriod
- Monthly and quarterly automated report generation from learner telemetry
- Reports persisted as JSON and markdown to `learner-reports/` directory
- `vortex_learn --report` flag for automated report generation
- `LearnerMetrics` extended with `total_evictions` field and `Default` derive
- `PersistentRunLog` and `LearnerMetrics` have `Default` implementations
- `cargo test --features learner --lib`: **155 passed, 0 failed** (up from 151)
- `cargo clippy --features learner -- -D warnings`: **0 errors**
- CHANGELOG entries per section

### KNOW-01 — Knowledge Acquisition System
- Added `learner` feature flag: compiled into release SKUs but RUNTIME-OFF by default
- Added `src/learner/` module: compliance, pipeline, state, curiosity, compile, api, scheduler, embeddings
- Added `vortex_learn` binary with `--help`, `--status`, `--sources`, `--run-once` flags
- Added 6 seed sources registry (Wikipedia, Wikibooks, Gutenberg, MDN, Rust Doc, Python Doc)
- Added compliance layer: HTTPS enforcement, robots.txt, token bucket, allowlist matcher, UA builder
- Added resumable state store with kill-resume support
- Added acquisition pipeline: fetch → extract → normalize → chunk → dedup → quality → embed
- Added curiosity engine: miss→gap→fetch→closed loop
- Added embeddings queue with priority ordering and cache
- Added hybrid RRF retrieval indexing into Kernel_02
- Added admin API endpoints for sources, status, gaps, licenses
- Added fragment compiler with SBOM-style manifest
- Added docs/LEARNER.md and docs/LEARNER_REPORT.md
- Added test fixtures for 5 standard pages and HTML extraction
- 91 library tests passing (0 failures)
- Added CHANGELOG entry
- Added resumable state store with kill-resume test support
- Added acquisition pipeline: fetch → extract → normalize → lang detect → chunk → dedup → quality → embed
- Added curiosity engine: miss→gap→fetch→closed loop
- Added embeddings queue with priority ordering and cache
- Added hybrid RRF retrieval indexing into Kernel_02
- Added admin API endpoints for sources, status, gaps, licenses
- Added fragment compiler with SBOM-style manifest
- Added docs/LEARNER.md and docs/LEARNER_REPORT.md
- Added test fixtures for 5 standard pages and HTML extraction
- Added CHANGELOG entry

### Security
- Bearer-token auth with `user`/`admin` roles (auto-generated, persisted
  user-scoped); all `/v1` endpoints except `/v1/health` and `/ws` require one.
- Strict same-origin CORS (explicit `allowed_origins` opt-in) replacing the
  permissive layer.
- Admin gate + JSONL audit log for `models/swap`, `knowledge/import`,
  `/v1/admin/*` (status / audit tail / token rotation); Security tab in the
  dashboard.
- Knowledge `file_path` imports sandboxed to allowlisted directories; model
  filename/repo validation; swap restricted to the pre-configured model
  matrix unless opted in; SHA-256 cache-integrity manifest + size cap.
- Input validation (temperature range, prompt/batch/import caps), WS
  admission control + token, per-IP rate limiting, `x-request-id`s,
  filesystem-error hygiene, optional TLS (`--tls-cert/--tls-key`).
- Fail-closed remote binds (non-loopback needs token auth + `--allow-remote`);
  `VORTEX_HOST/PORT/API_TOKEN/ADMIN_TOKEN/ALLOW_REMOTE` env wiring.
- Frontend: same-origin token bootstrap (memory-only), `?token=` WebSocket
  auth, retries limited to idempotent GETs (no POST/503 replay).
- Second wave: markdown link allowlist (XSS), server WS bounds, DPAPI
  sessions/tokens + ACLs + retention, single-instance mutex, graceful
  shutdown, self-hosted fonts, CSP/security headers, `--read-only`,
  loopback-only admin, hot-reload, metrics, session purge, body caps,
  disk-space guard, release provenance/SBOM/checksums.
- Quality gates: `cargo fmt` clean, zero `clippy` warnings (incl. 24
  pre-existing), `oxlint` zero errors, JSON 404 for unknown API paths,
  manual token paste for remote UI, reproducible-artifact + log-hygiene CI.

### Added
- PERF-01 Section 1: opt-in CPU SKUs (`tools/build_skus.ps1`), `--simd-probe`,
  `compiled_features` in `/v1/device`, SKU-aware launcher selection, and
  `VORTEX_INFER_THREADS`/`async_workers` topology (default async workers: 2).
- PERF-01 Section 2: `tools/setup_dev_exclusions.ps1` (idempotent Defender
  exclusions for model cache + SKU binaries), background prefault thread
  (configurable 1 MiB stride, non-blocking health), `--high-priority` flag
  (Windows `ABOVE_NORMAL_PRIORITY_CLASS`), prefault bench + tests.
- PERF-01 Section 3: CPU tier detection (`avx2`/NEON gate) + tier-aware
  `generation_defaults` advertised on `/v1/models`; economy/quality model
  matrix (`eco` = SmolLM2-135M Q4_0, `q4_0` = Qwen2.5-0.5B Q4_0) with
  SHA-256 manifest verification in the swap path; greedy fast path
  (`temperature <= 1e-7` now selects candle ArgMax — fixing a latent
  divide-by-zero softmax — and samples via a single-vector manual argmax
  with in-place repeat penalty, skipping all top-k/top-p machinery);
  optional user-facing model routing (`security.allow_model_routing`,
  fail-closed off, audited as `models.route`); `max_seq_len` now reflects
  the active engine instead of a constant.
- Error Boundaries for graceful error handling
- Loading Skeletons for better UX
- Connection status indicator
- Network status monitoring
- Keyboard shortcuts (Ctrl+N, Ctrl+Enter, Ctrl+Shift+L, Ctrl+Shift+D)
- API client with timeout and retry logic
- WebSocket reconnection with exponential backoff
- Message queuing for offline support
- Focus trap for modals
- Theme options (animations toggle, compact mode)
- Code splitting and lazy loading for routes
- GitHub issue and PR templates
- CI/CD pipeline with GitHub Actions
- Release automation for multi-platform binaries
- Docker and Docker Compose configuration
- Comprehensive documentation (API, User, Developer, Architecture guides)
- Frontend tests with Vitest and Testing Library
- Backend unit tests
- Security policy
- Contributing guidelines
- Code of conduct

### Changed
- Improved API client with AbortController and retry logic
- Enhanced WebSocket service with heartbeat/ping-pong
- Better error handling throughout the application
- Updated theme store with persistence and migration

### Fixed
- Layout nesting issue (RTLLayout duplication in Dashboard)
- ModelsTab crash when API returns single object instead of array
- Toast component duplicate exports
- Card component duplicate exports

### Documentation
- Added API_REFERENCE.md with all endpoints documented
- Added USER_GUIDE.md for end users
- Added DEVELOPER_GUIDE.md for contributors
- Added ARCHITECTURE.md for system design

---

## [0.1.0] - 2026-08-08

### Added
- Initial release of Vortex Atoms AI
- 5-Kernel Matrix architecture (UI, Router, Code/Logic, Multimodal, Supervisor)
- HTTP API server with Axum
- WebSocket support for streaming
- Knowledge base with vector search
- Bio-medical, Engineering, Finance, and Humanities knowledge modules
- Model loading with memory mapping
- Tool execution system
- MCP (Model Context Protocol) support
- Plugin system
- Memory consolidation
- Agent loop for multi-step reasoning
- React 19 frontend with Tailwind CSS v4
- RTL support for Arabic
- i18n (Arabic/English)
- Dark/Light theme support
- Zustand state management
- React Router v7

---

## Release Notes Template

### [X.Y.Z] - YYYY-MM-DD

#### Added
- New features

#### Changed
- Changes to existing functionality

#### Deprecated
- Soon-to-be removed features

#### Removed
- Removed features

#### Fixed
- Bug fixes

#### Security
- Security improvements
