// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# AGENT Initial Architecture — Vortex Atoms AI (as-is, 2026-09-20)

> Stage 0 discovery. Describes the repository AS IT CURRENTLY EXISTS.
> No proposed changes in this document.

## 1. Repository layout

- `Cargo.toml`: single-crate package `vortex_atoms_ai` v0.2.0, edition 2021,
  rust-version 1.78. `[workspace]` resolver 2 but no members (single crate).
- `src/`: ~50 modules. Core: `lib.rs` re-exports kernel, 5-kernel matrix,
  knowledge, inference, security, API.
- `src/bin/`: `vortex_api.rs`, `vortex_chat.rs`, `vortex_dashboard.rs`
  (feature `dashboard`), `vortex_learn.rs` (feature `learner`).
- Features (7): `dashboard` (eframe), `cuda`, `par-matmul` (rayon),
  `neural-embed` (ort), `tray`, `ggml` (llama-cpp-2), `learner`
  (redb, whatlang, scraper, robots_txt, tokio-util, tempfile).
- `frontend/`: React 19 + TypeScript + Vite + Tailwind + i18next + zustand
  + tanstack query. Vitest unit tests, Playwright E2E. Arabic-first RTL.
- `knowledge/`: `.source.md` + compiled `.tcz`/`.bin` fragments +
  `avian_genetics_fastpath.json`; subdirs `custom/ engineering/ finance/
  humanities/ medical/`.
- `tests/`: `unit_tests.rs`, `integration_tests.rs`, `fixtures/`, `golden/`.
- `docs/`: API_REFERENCE, ARCHITECTURE, SECURITY, data_room (facts.json +
  _render.py + out/), KNOWN_ISSUES, PERF_REPORT, LEARNER_REPORT.
- Config: `vortex.json` (currently learner-only JSON).
- CI: `.github/workflows/ci.yml` (backend, frontend, data-room, docker,
  security), `release.yml`. Release v0.3.0.
- Install/dist: `Dockerfile`, `docker-compose.yml`, `installer.nsi`,
  `dist/`, `SBOM.json`, `provenance.json`, `SHA256_MANIFEST.txt`.

## 2. Runtime architecture

- `VortexAtomsKernel` (`kernel.rs`): mmap weights via `MmapWeights`,
  materialize single tensor slices into Candle. CPU-only DevicePreference.
- `LlmInference` (`llm_inference.rs`, ~833 lines): GGUF load via
  `candle_core::quantized::gguf_file`, tokenizer, `VortexSampler`,
  `VortexCache`, conversation history (max 20 turns), reused logits
  scratch buffers, layer prefetch flag, KV window/prefix cache flags,
  optional `SpeculativeDecoder` + `NGramDrafter`.
- Memory model: model file memory-mapped for prefault; tensors
  materialized on demand. mmap != zero-RAM claim; resident/materialized
  distinction exists in code but is not benchmarked per-tensor here.
- Speculative decoding (`speculative.rs`, ~324 lines): `SpeculativeDrafter`
  trait, `NGramDrafter` (Markov n-gram from context), `SpeculativeDecoder`
  accept/reject. Correctness scope for sampling modes is NOT documented
  in-code; treat as experimental until Stage 8 verifies.

## 3. 5-Kernel Matrix (as-is)

- `five_kernel_matrix.rs` (~248 lines): spawns 5 Tokio tasks, mpsc ingress
  (capacity 256 default), broadcast events (512), shared state
  `Arc<RwLock>` via `new_shared_state(memory_budget 256 MiB default).
- Optional shared `Arc<Mutex<LlmInference>>` injected when `llm_config`
  present; load failure logs + continues with None (degraded mode).
- Kernels:
  - K01 `kernel_01_ui_interaction.rs`: UI input intake.
  - K02 `kernel_02_router_vector_db.rs`: intent routing + vector DB.
  - K03 `kernel_03_code_logic_expert.rs`: code/logic + LLM generate route.
  - K04 `kernel_04_multimodal_media.rs`: media rendering.
  - K05 `kernel_05_supervisor_watchdog.rs`: supervision, retention,
    purge, health.
- IKC (`ikc.rs`): `KernelId`, `KernelCommand` (UiInput, RouteIntent,
  ExecuteLogic, RenderMedia, RegisterKnowledgeFragment, PurgeInactive,
  Shutdown, LlmGenerate, LlmCancel), `IkcEvent` broadcast.

## 4. LLM / inference / GGUF path

- Entry: `vortex_api` boots `LlmInference::load(&LlmConfig)` from
  `--model/--repo/--tokenizer` or defaults (Qwen2.5-0.5B GGUF ~469 MB,
  cached in `%LOCALAPPDATA%/vortex_atoms_ai/models`).
- `llm_model.rs`: `detect_architecture`, `detect_eos_token_id`,
  `load_gguf_model`, `LlmModelEnum`.
- `llm_sampling.rs`: `VortexSampler` (temperature/top-k/top-p surface).
- `llm_prompt.rs`: `format_chat_history`, `ChatTemplate`.
- `llm_stream.rs`, `llm_cache.rs` (`VortexCache`), `token_cache.rs`
  (`HotTokenCache`), `perf_topology.rs`, `model_ladder.rs`.
- GGML path: feature `ggml` via `llama-cpp-2` (native); blocked notes in
  `docs/PERF-02_GGML_BLOCKED.md` — do not assume working.

## 5. Knowledge / RAG / embedding path (as-is)

- `KnowledgeImporter` (`knowledge_import.rs`, ~165 lines): `chunk_text`
  (byte slicing — Stage 2 must verify UTF-8 safety), `import_text`,
  `import_file` (read_to_string, file-stem id), `import_directory`
  (extension allowlist: txt/md/rs/py/js/ts/json/toml/yaml/html/css/c...).
  No ImportReport; single error aborts batch.
- `VectorStore` (`llm_embed.rs` 93-180): in-memory `Vec<(id, vec, text)>`,
  `Mutex<Box<dyn NeuralEmbedder>>`. Retrieval is linear scan with cosine
  similarity — O(N). No ANN index.
- `EmbeddingModel::embed`: FNV-1a hash per whitespace token into fixed
  dims, signed magnitude 1/(1+i), L2-normalized. **Hash-based, NOT neural
  semantic.** Implements `NeuralEmbedder` trait (naming is misleading;
  Stage 4 must document accurately, not silently rename API).
- `knowledge_orchestrator.rs` (~465 lines): `.tcz`/`.bin` fragment
  registry, `deterministic_embedding` of semantic hints, load/evict
  actions, `KnowledgeLoadReport`.
- Compiled modules: bio_medical, engineering_expert, finance_math,
  global_humanities (source.md + .tcz/.bin), avian fastpath JSON.
- Learner (`src/learner/`, 15 files, feature-gated): pipeline, scheduler,
  compliance, curiosity, eval, telemetry. Disabled by default
  (`vortex.json` learner.enabled=false).

## 6. API / WebSocket / auth path

- `llm_api.rs`: axum Router. `/v1/generate`, `/v1/chat`, `/v1/batch`,
  `/v1/models`, `/v1/models/swap`, `/v1/health`, `/v1/device`,
  `/v1/bench`, `/v1/tools`, `/v1/tools/execute`, `/v1/tools/call`,
  `/v1/knowledge/search`, `/v1/knowledge/import`, `/v1/embeddings`,
  `/v1/metrics`, `/v1/admin/*` (status/audit/rotate/reload/metrics),
  `/auth/bootstrap`, `WS /ws` (`llm_ws.rs`).
- `security.rs` (~1491 lines): bearer auth (user/admin roles),
  auto-generated persisted tokens, strict same-origin CORS + allowlist,
  per-IP rate limit, request IDs, audit log, path sandboxing for imports
  and model filenames, input validation. `public_message()` redacts
  filesystem paths. Fail-closed defaults.
- `session_store.rs`, `state.rs` (`SharedState`), `llm_agent.rs`,
  `llm_tools.rs`, `llm_mcp.rs`, `llm_memory.rs`, `llm_structured.rs`,
  `llm_plugin.rs`: agent/tool/MCP/memory surfaces (Stage 11 audits).

## 7. Frontend architecture

- `App.tsx`, `main.tsx`, `i18n/` (ar/en), `stores/` (zustand),
  `services/` (api, ws), `components/` (chat, dashboard, layout, ui),
  `hooks/`, `utils/`, `types/`.
- RTL: Arabic-first, dir switching by language.
- Tests: vitest (356), Playwright E2E (54). Lint: oxlint strict.
- Build: `tsc -b && vite build`; served embedded via `rust-embed`
  (`embedded_frontend.rs`) — backend CI builds frontend first.

## 8. Tests / benchmarks / release

- Rust: 156 lib tests (`cargo test --features learner --lib`).
  Integration: `tests/integration_tests.rs` incl. known flake
  `test_five_kernel_matrix_submit_ui_input` (marked `#[ignore]`,
  SYNC-04 diagnosis).
- Frontend: 356 vitest, 54 E2E (3 Firefox flakes green-on-retry,
  KNOWN_ISSUES + issues #1-#3).
- Coverage: 99.49% statements (frontend).
- Bench: `perf_topology` harness via `/v1/bench`; PERF_REPORT frozen
  baseline. No `benches/` criterion suite as-is.
- Release: tag-triggered `release.yml`, SBOM 715 components, provenance,
  SHA-256 sidecars, `verify_release.ps1`.

## 9. Current modifications / safety

- `git status`: clean. Branch `main`, in sync with `origin/main`.
- No user uncommitted changes to protect. No destructive ops performed
  in Stage 0 (read-only).
- HEAD: 4555f3d (fix: restore _render.py).

## 10. Major execution paths (summary)

1. HTTP chat: `POST /v1/chat` → auth → K01 UiInput → K02 route →
   K03 LLM generate (`Arc<Mutex<LlmInference>>`) → stream/SSE or JSON.
2. WS chat: `/ws` → same kernel path, token stream events.
3. Embeddings: `POST /v1/embeddings` → hash embedder (or ORT neural
   when `neural-embed` enabled — Stage 4 verifies).
4. Knowledge search: `POST /v1/knowledge/search` → VectorStore linear
   cosine top-k.
5. Knowledge import: `POST /v1/knowledge/import` → sandbox check →
   KnowledgeImporter chunk+insert.
6. Model swap: `POST /v1/models/swap` (admin) → reload GGUF.
7. Learner (opt-in): scheduler → pipeline → compliance → redb store.
