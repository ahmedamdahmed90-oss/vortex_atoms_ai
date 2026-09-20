# Vortex Atoms AI

[![CI](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/actions/workflows/ci.yml/badge.svg)](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ahmedamdahmed90-oss/vortex_atoms_ai)](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.78+-orange.svg)](https://www.rust-lang.org/)
[![Frontend](https://img.shields.io/badge/React-19-61DAFB.svg)](https://react.dev/)

**A full-stack, locally-run AI assistant with complete data sovereignty.**

Built with Rust for performance, React for user experience, and Arabic-first RTL support.

## Key Features

| Feature | Description |
|---------|-------------|
| **Local-First** | All data stays on your machine. No cloud dependency. |
| **Fast Inference** | Rust-native GGUF pipeline with memory-mapped models |
| **Arabic RTL** | Full right-to-left interface support |
| **5-Kernel Matrix** | Modular architecture: UI, Router, Code, Media, Watchdog |
| **Security Hardened** | 11 threats addressed, SBOM, provenance, SHA-256 |
| **Dashboard** | Real-time monitoring and configuration |

## Quick Start

### Option 1: Download Release (Recommended)

1. Download from [GitHub Releases](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/releases/tag/v0.2.0)
2. Extract `vortex-atoms-ai-windows-amd64.zip`
3. Run `vortex_api.exe`

### Option 2: Build from Source

```bash
git clone https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai.git
cd vortex_atoms_ai
cargo build --release --bin vortex_api
./target/release/vortex_api.exe
```

### Option 3: Docker

```bash
docker build -t vortex-atoms-ai .
docker run -p 8080:8080 -e VORTEX_API_TOKEN=your-token vortex-atoms-ai
```

## Documentation

| Document | Description |
|----------|-------------|
| [API Reference](docs/API_REFERENCE.md) | Complete API documentation |
| [Architecture](docs/ARCHITECTURE.md) | System design and 5-Kernel Matrix |
| [Security](SECURITY.md) | Security model and best practices |
| [Developer Guide](docs/DEVELOPER_GUIDE.md) | Build, test, and contribute |
| [User Guide](docs/USER_GUIDE.md) | End-user documentation |

`vortex_atoms_ai` is a full-stack, locally-run AI assistant: a Rust workspace with a low-latency
machine-learning kernel, an asynchronous 5-Kernel Matrix runtime, a native CUDA-free GGUF inference
pipeline, and an OpenAI-compatible HTTP/WebSocket API server — plus an Arabic-first (RTL)
React + TypeScript frontend with a management dashboard.

## Quick Start

### Run the API server

```bash
cargo build --release --bin vortex_api
./target/release/vortex_api.exe
```

On first boot the server downloads the default GGUF model (`Qwen/Qwen2.5-0.5B-Instruct-GGUF`,
`qwen2.5-0.5b-instruct-q4_k_m.gguf` ~469 MB) plus its tokenizer and caches them in
`%LOCALAPPDATA%\vortex_atoms_ai\models` (override the model with `--repo`, `--model`, `--tokenizer`);
the model is then memory-mapped and loaded once at startup.

```bash
./target/release/vortex_api.exe --help            # usage
./target/release/vortex_api.exe --init-config     # write a default vortex.json
./target/release/vortex_api.exe --port 8080 --temperature 0.7
```

The server listens on `http://127.0.0.1:8080` and exposes OpenAI-compatible `/v1/*` endpoints plus a
WebSocket stream at `/ws`:

```text
POST /v1/generate          POST /v1/chat            POST /v1/batch
GET  /v1/models            GET  /v1/health          GET  /v1/device
GET  /v1/tools             POST /v1/tools/execute   POST /v1/tools/call
POST /v1/models/swap       POST /v1/knowledge/search POST /v1/knowledge/import
POST /v1/embeddings        WS   /ws                 (streaming chat)
```

Quick smoke test:

```bash
curl http://127.0.0.1:8080/v1/health
curl http://127.0.0.1:8080/v1/models
```

### API runtime behaviour

The API bounds worst-case response time so the server never wedges behind long generations:

- **Concurrency guard** — at most `MAX_CONCURRENT_INFERENCE` (2) blocking inferences run at once.
  Excess requests are rejected instantly with `503 {"error": "engine busy: retry later"}` via a
  `tokio::sync::Semaphore`; `block_in_place` keeps synchronous inference off the async workers.
- **Bounded tokens** — every inference route clamps the requested `max_tokens` to `MAX_API_MAX_TOKENS`
  (512). `/v1/models` reports `"max_generation_tokens": 512`. This caps worst-case latency per request.
- **Live observability** — `/v1/health`, `/v1/device`, `/v1/models` use `try_read()`: while a
  generation holds the engine, they answer `503 {"status": "busy"}` in a couple of milliseconds rather
  than queueing behind the request.
- **Fast error paths** — malformed bodies are rejected in ~2 ms before any engine work
  (e.g. `422` for a missing `messages` field).

### Hardware / performance notes

Measured on the reference dev machine (4 cores, **no AVX2**, SIMD `AVX + SSE4.2 + SSE4.1 + SSSE3 + SSE2`):
a tiny 4-token generation was still running after 8 minutes while pegging ~2 cores. The engine,
the bounded pipeline, and observability are correct and resilient, but **interactive-speed generation
is not achievable without AVX2/AVX512 (or a GPU)**. The 512-token cap and the instant `503 busy`
semantics above exist precisely to keep the server usable on slower hardware.

**CPU SKUs (measured, not assumed):**

| SKU | Build flags | Use when host has | Default? |
|---|---|---|---|
| `vortex_api.exe` (default) | repo `.cargo/config.toml`: `target-cpu=native` | the build host's CPU class | **yes** |
| `vortex_api-sse41.exe` | `+sse4.1,+ssse3` | SSE4.1 | no |
| `vortex_api-avx1.exe` | `+avx,+sse4.2,+sse4.1,+ssse3` | AVX | no |

Honest correction: the default artifact was never pure SSE2 — `.cargo/config.toml`
pins `target-cpu=native`, so a default build is tuned for its build host (on the
reference machine the default binary probes as `sku=avx1`). The default build
command and config are intentionally untouched. For a truly portable SSE2-only
binary, build explicitly with `RUSTFLAGS="-C target-cpu=x86-64"`.

Build the opt-in SKUs with `tools/build_skus.ps1` (separate target dirs; default cache untouched).
`start-server.ps1` probes `vortex_api-avx1.exe` then `vortex_api-sse41.exe` with `--simd-probe`
and falls back to baseline when no SKU is present/compatible. `/v1/device` reports host `simd`
plus `compiled_features`.

**Generation economics (PERF-01 Section 3):** the engine detects a CPU tier — hosts without
AVX2/NEON are `low`, everything else `standard` — and advertises tier-aware defaults in
`/v1/models.generation_defaults` (`low`: greedy `temperature: 0.0`, 1024-token context,
`eco` model; `standard`: sampled, 4096, `q4_0`). Greedy decoding (`temperature <= 1e-7`, which
also fixed a latent divide-by-zero softmax) now runs a single-vector manual argmax with an
in-place repeat-penalty window, skipping candle's softmax/top-k/top-p allocations entirely.
A pre-configured economy/quality matrix ships with the server — `eco`
(SmolLM2-135M Q4_0, ~77 MB) and `q4_0` (Qwen2.5-0.5B Q4_0) — both swap-allowlisted and
SHA-256 manifest-verified. User-facing model routing is **off by default**
(`security.allow_model_routing`); when enabled, `/v1/generate` and `/v1/chat` accept a
`model: "eco"|"q4_0"` hint (audited as `models.route`).

**Live generation round-trip (this session, release build):** a real `/v1/generate`
(`{"prompt":"2 + 2 =","max_tokens":8,...}`) completed in **549.5 s HTTP 200** and returned an actual
model response (`The score provided is **0.0`, `total_tokens: 8`). The engine consumes ~1.0 core
continuously while generating (measured 5.2 s CPU per 5 s wall). Throughout the entire sluggish
generation the server stayed fully responsive — `/v1/health` and `/v1/device` answered in **1.5–2.9 ms**
with `503 {"status":"busy"}` while the engine was held, then recovered to `status: ok` instantly
after completion with no leak or restart. The 2-permit semaphore did not wedge: a second concurrent
request was rejected fast rather than queued forever.

**Live WebSocket streaming (this session, same release server):** a real browser WebSocket against
`ws://127.0.0.1:8080/ws` received the server's `{"type":"ready"}` handshake, then streamed actual
token events (`{"type":"token","text":"OK",...}`) and a final `{"type":"done","total_tokens":1,...}`
— completing one real token in 394 s on this hardware. The `ready` handshake, token streaming, and
terminal `done` event all match the protocol documented in `docs/API_REFERENCE.md`.

### Run the web frontend

```bash
cd frontend
npm install
npm run dev
```

Open `http://localhost:5173` — an Arabic-first (RTL) chat interface with a full management dashboard.
The chat generation panel exposes temperature and a `max_tokens` slider clamped to `64..512`
(default 512), matching the API ceiling.

### Standalone Windows package

The frontend is **embedded into the server binary** (`rust-embed`): `vortex_api.exe` is a
single-file server that serves the full Arabic (RTL) web UI, the REST API, and the WebSocket
stream from one process on one port. In production the UI calls the API same-origin (relative
`/v1`), so it works on any port without configuration.

Distribution artifacts (all under `dist/`):

```text
vortex-atoms-ai-app/            Complete runtime: exe + models/ + knowledge/ + vortex.json + scripts
  vortex_api.exe                15.6 MB — API (14 endpoints) + embedded web UI + WS streaming
                                (+ optional system tray: `--tray` icon with Open/Stop/Exit menu)
  vortex_chat.exe               9.1 MB — interactive CLI chat
  vortex_dashboard.exe         10.9 MB — optional native egui dashboard
  models/                       GGUF (~469 MB) + tokenizer — pre-bundled, no first-run download
  vortex.json                   host 127.0.0.1 / port 8080
  start-server.ps1              seeds the model cache (if needed) and launches the server
  stop-server.ps1               stops a running server
vortex-atoms-ai-win/            Standalone EXEs only (no model) — API + CLI chat + dashboard
VortexAtomsAI-Portable.zip      Portable ZIP of the full app (~470 MB)
installer.nsi                   NSIS installer source → dist\VortexAtomsAI-Setup.exe
```

The binaries are profile-optimized release builds (`opt-level=3`, thin LTO, stripped, panic=abort),
compiled with the `tray` feature so `--tray` works. Verified live this session: the packaged
`vortex_api.exe` booted standalone from a fresh directory (no project files present) and
`/v1/health` + `/v1/models` returned 200 with SIMD detected; then a real headless Chromium loaded
the embedded UI from the exe and rendered the dashboard **Overview** (live relative `/v1/health`:
architecture `llama`, SIMD string, 10 knowledge chunks) and the **Models** tab
(`llama • 4,096 seq • 512 gen`) with **0 console errors** — everything in-process.

**System tray** (`src/system_tray.rs`, feature `tray`): run `vortex_api.exe --tray` and an atom
glyph icon sits in the notification area with a context menu — *Open Dashboard* (default browser,
also on left-click), *Stop server* (graceful shutdown via a watch channel), and *Exit*. The icon is
generated locally as 32x32 RGBA (no asset files needed). Fixed a real bug this session:
`tray-icon` on Windows does **not** pump messages itself, so the original `try_recv + sleep`
loop never dispatched `WM_USER_TRAYICON` and clicks were swallowed. `run_loop` now runs a
real Win32 message pump (`GetMessageW`/`TranslateMessage`/`DispatchMessageW`) on the tray
thread (feature `tray` pulls in optional `windows-sys`). Verified live: after the pump fix a
click message dispatched through `tray_proc` → `TrayIconEvent::Click` → `open_browser`, which
really launched the default browser (Brave) on the dashboard URL. Note: the library only
emits click events when the icon's rect is resolvable (`Shell_NotifyIconGetRect` → S_OK), so
an icon collapsed into the overflow flyout must be shown first (one chevron click) — this is
the library's own gate, not ours.

The installer is built with a portable [NSIS 3+](https://nsis.sourceforge.io/Download) kept at
`%LOCALAPPDATA%\nsis\nsis-3.10\makensis.exe` (no admin/UAC needed to compile or to install the app):

```powershell
& "$env:LOCALAPPDATA\nsis\nsis-3.10\makensis.exe" installer.nsi   # → dist\VortexAtomsAI-Setup.exe
```

`dist\VortexAtomsAI-Setup.exe` (~462 MB) was built from the tray-enabled binaries and **verified
end-to-end this session**: silent install → exit 0 (exe + models + knowledge + config installed
under `%LOCALAPPDATA%\VortexAtomsAI`, model seeded automatically into the engine cache so no
first-run download; shortcuts launch the server with `--port 8080 --tray`) → the installed
`vortex_api.exe --tray` launched, reported "System tray enabled", and answered `/v1/health` 200
(10 knowledge chunks) with the embedded UI 200 on a fresh port → silent uninstall → exit 0,
directory removed. Installed and uninstalled twice with the same clean result. After the Win32
message-pump fix the tray was re-verified end-to-end (left-click really opens the default
browser); `dist\VortexAtomsAI-Setup.exe` (~462 MB) and `dist\VortexAtomsAI-Portable.zip`
(~475 MB) were rebuilt from the fixed binaries. Mid-verification a packaging bug was
caught: the installer packs `dist\vortex-atoms-ai-app\vortex_api.exe` (not the
`vortex-atoms-ai.exe` alias), and that file had not been refreshed after the pump fix —
so the first rebuilt Setup still contained the old click-broken exe (verified: installed
exe did not open the browser). Fixed by copying the fixed exe to both names in both dist
trees and rebuilding Setup + ZIP; re-verified: silent install → installed exe SHA‑matches
the fixed build → health 200 + tray enabled on the installed copy → silent uninstall → exit
0 → directory gone. The remaining "click opens browser" re-check on the installed exe
could not be reproduced visually later because this automated session lost the interactive
tray (Shell_NotifyIconGetRect returns S_FALSE/E_FAIL for every id while Explorer's tray is
collapsed/unavailable); the same click chain did open Brave on the identical fixed binary
earlier, so behavior is verified on the exact shipped executable.
Human-verified by the operator on the live desktop: right-click → tray menu
renders (Open Dashboard / Stop server / Exit) and **Stop server performs a
graceful shutdown** (confirmed twice via server log); every tray gesture is
now logged with a `[Tray]` prefix for future forensics.

### Docker

```bash
docker compose up --build
```

## Foundation

- `candle-core` and `candle-nn` for native Rust tensor and neural-network execution.
- `qdrant-client` for Qdrant-compatible vector point structures and vector search integration.
- `tokio` for async service runtime and the 5-Kernel Matrix tasks.
- `memmap2` for read-only memory-mapped model weights.
- `serde` for configuration, IKC messages, and health/readiness payloads.

## Security & Control Privileges

Local-first, fail-closed (`src/security.rs`):

- **Bearer roles**: auto-generated `user` (`vxt_…`) / `admin` (`vxa_…`) tokens
  (persisted user-scoped in `%LOCALAPPDATA%\vortex_atoms_ai\auth.json`);
  everything except `/v1/health`, static assets and the loopback-only
  `/auth/bootstrap` requires one. The web UI bootstraps both automatically.
- **Strict CORS** (same-origin by default, `security.allowed_origins` opt-in),
  per-IP rate limiting, `x-request-id` on every response.
- **Admin gate + audit**: `models/swap`, `knowledge/import` and
  `/v1/admin/*` (status / audit tail / token rotation) need the admin token;
  every privileged action is JSONL-audited. A **Security tab** in the
  dashboard shows the live posture and the audit tail.
- **Sandboxing**: knowledge `file_path` imports confined to allowlisted
  directories; model filenames/repo ids validated; swaps limited to the
  pre-configured model matrix unless `allow_custom_models`; SHA-256
  cache-integrity manifest; temperature/prompt/batch/import caps.
- **Fail-closed binds**: non-loopback `--host` refuses to start without token
  auth + `--allow-remote`; optional `--tls-cert/--tls-key` (HTTPS) for LAN.
- Env overrides: `VORTEX_HOST/PORT/API_TOKEN/ADMIN_TOKEN/ALLOW_REMOTE`.
- **Second wave** (SEC-01 done):
  - ✅ model-output link allowlist (no `javascript:` XSS)
  - ✅ server-side WS idle/ping/size bounds
  - ✅ DPAPI-encrypted sessions + tokens with user-only ACLs and 30-day retention
  - ✅ single-instance mutex with exit code 2 + `tray_flash`
  - ✅ graceful Ctrl+C/tray shutdown
  - ✅ self-hosted fonts (zero third-party requests)
  - ✅ strict CSP + security headers middleware
  - ✅ `--read-only` kiosk mode
  - ✅ loopback-only admin
  - ✅ hot-reload (`/v1/admin/reload`) with 422 validation
  - ✅ metrics (`/v1/metrics` Prometheus + `/v1/admin/metrics` JSON)
  - ✅ session purge with confirm dialog + startup retention sweep
  - ✅ per-route body caps (1MB/2MB/50MB)
  - ✅ disk-space guard (1024 MB default)
  - ✅ SBOM/provenance/SHA-256 sidecars on release
  - **Deferred**: Authenticode signing (requires code-signing certificate), non-Windows DPAPI

## Vortex Dashboard

The optional native dashboard uses `egui`/`eframe` with the lightweight Glow backend:

```bash
cargo run --features dashboard --bin vortex_dashboard
```

Dashboard modes:

1. Intelligent Chat & Scenario Writing.
2. Live Preview Panel for generated apps, websites, and games.
3. Media Processing Windows for generated image/audio/video frame streams.

Every dashboard action is sent directly to `Kernel_01` through the existing Tokio `mpsc` IKC channel. The dashboard keeps bounded chat logs, event logs, preview source, and media-frame metadata, with a strict 50 MiB UI-state budget and explicit trimming of inactive interface data.

## 5-Kernel Matrix

The runtime spawns five independent Tokio tasks:

1. `Kernel_01` — UI & Interaction: manages user-facing input and interaction state.
2. `Kernel_02` — Router & Vector DB: performs semantic intent routing through a local in-memory Qdrant-compatible vector database.
3. `Kernel_03` — Code & Logic Expert: executes programming and logic modules only.
4. `Kernel_04` — Multimodal & Media: renders image/audio/video/multimodal work frame by frame.
5. `Kernel_05` — Supervisor & Watchdog: purges inactive knowledge fragments and explicitly calls `std::mem::drop` on removed fragments.

Inter-Kernel Communication uses directed Tokio `mpsc` channels and fan-out Tokio `broadcast` events. Shared state is protected with `Arc<RwLock<VortexAtomsSharedState>>`.

```rust
use vortex_atoms_ai::{FiveKernelMatrix, FiveKernelMatrixConfig};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let matrix = FiveKernelMatrix::spawn(FiveKernelMatrixConfig::default());
let mut events = matrix.subscribe_events();

matrix
    .submit_ui_input("session_01", "render a 24-frame video of vortex atoms")
    .await?;

matrix.request_purge(0).await?;
matrix.shutdown().await;
# Ok(())
# }
```

## Kernel_02 vector DB and dynamic knowledge orchestration

Kernel_02 now uses an in-memory Qdrant-compatible vector database. The hot path remains local and low-latency, while the index can export `qdrant_client::qdrant::PointStruct` snapshots for compatibility with a real Qdrant deployment.

Kernel_02 also orchestrates compressed cognitive fragments on demand:

- Registers `.tcz` and `.bin` fragment descriptors with semantic hints.
- Detects semantic intent through the vector index.
- Loads matching compressed files only on cache misses.
- Unloads non-matching or inactive resident fragments.
- Keeps frequently requested tokenized cognitive data in a hot token cache.

The token cache is a zero-cost abstraction: the `TokenCache<K>` trait is intended for static dispatch, and hot blocks are shared through `Arc<[u32]>`. A cache hit clones only an `Arc` handle and avoids disk reads.

```rust
use vortex_atoms_ai::{
    FiveKernelMatrix, FiveKernelMatrixConfig, KnowledgeFragmentDescriptor,
};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let matrix = FiveKernelMatrix::spawn(FiveKernelMatrixConfig::default());

matrix
    .register_knowledge_fragment(KnowledgeFragmentDescriptor::new(
        "logic_fast_path",
        "code_logic",
        "rust code logic module execution symbolic reasoning",
        "knowledge/logic_fast_path.bin",
    ))
    .await?;

matrix
    .submit_ui_input("session_02", "run a rust logic module")
    .await?;

matrix.shutdown().await;
# Ok(())
# }
```

## Global humanities extension

The project includes a compiled high-density binary fragment:

- `knowledge/global_humanities_module.source.md`
- `knowledge/global_humanities_module.bin`
- `src/global_humanities_module.rs`

Kernel_02 detects grammar, translation, world-language dictionary, global history, geography, and GIS/mapping intents. It routes these requests to Kernel_03 as:

```rust
module = "global.humanities"
```

Kernel_03 loads `knowledge/global_humanities_module.bin` read-only with `memmap2`, parses its compact length-prefixed binary sections with checked slicing, answers from the mapped extension, and explicitly drops the mmap after the request:

```rust
let module = GlobalHumanitiesModule::load_default()?;
let answer = module.query(prompt)?;
std::mem::drop(module);
```

The compiled module is currently 6,562 bytes, far below the 50 MiB mapping budget. Recompile it with:

```bash
python3 tools/compile_global_humanities_module.py
```

## Finance and mathematical strategy extension

The project includes a compiled compact TCZ fragment:

- `knowledge/finance_math_module.source.md`
- `knowledge/finance_math_module.tcz`
- `src/finance_math_module.rs`

Kernel_02 detects macro/microeconomic, corporate finance, factory logistics, risk, statistics, forecasting, and strategic-planning intents. It routes them to Kernel_03 as:

```rust
module = "finance.math"
```

The module is indexed in the active Vector DB layer through the default knowledge registry and qdrant-compatible point snapshots. Kernel_03 mmap-loads and explicitly drops it after use to avoid runtime leaks.

## Bio-medical and avian-genetics extension

The project includes a compiled compact TCZ fragment:

- `knowledge/bio_medical_module.source.md`
- `knowledge/bio_medical_module.tcz`
- `src/bio_medical_module.rs`

Kernel_02 detects biomedical and avian-genetics intents such as human general medicine, clinical pharmacology, veterinary science, avian medicine, budgie genetics, finch genetics, Hagoromo, Blackwing, Opaline, and Rainbow mutation probability logic. It routes the request to Kernel_03 as:

```rust
module = "bio.medical"
```

Kernel_03 loads `knowledge/bio_medical_module.tcz` read-only with `memmap2`, validates the module is below the 50 MiB mapping budget, answers from the mapped extension, and explicitly drops the mmap after the request:

```rust
let module = BioMedicalModule::load_default()?;
let answer = module.query(prompt)?;
std::mem::drop(module);
```

The compiled module is currently 7,610 bytes, far below the 50 MiB address-space mapping limit. Recompile it with:

```bash
python3 tools/compile_bio_medical_module.py
```

Safety boundary: the module is educational decision support. It does not diagnose, prescribe, provide patient-specific dosing, replace licensed clinicians/veterinarians, or replace emergency care.

## Engineering expert extension

The project includes a compiled compact TCZ extension:

- `knowledge/engineering_expert_module.source.md`
- `knowledge/engineering_expert_module.tcz`
- `src/engineering_expert_module.rs`

Kernel_02 detects engineering intents such as electronics repair, PCB diagnostics, lithium battery recycling algorithms, metal detector schematic modeling, and precious-metal process-safety analysis. It then routes the request to Kernel_03 as:

```rust
module = "engineering.expert"
```

Kernel_03 loads `knowledge/engineering_expert_module.tcz` read-only with `memmap2`, answers from the mapped extension, and explicitly drops the mmap after the request:

```rust
let module = EngineeringExpertModule::load_default()?;
let answer = module.query(prompt)?;
std::mem::drop(module);
```

The extension is safety-bounded: it supports diagnostics, modeling, sorting algorithms, hazard recognition, compliance, and certified-refiner handoff logic. It intentionally excludes operational recipes, reagent concentrations, reaction conditions, or hands-on instructions for dangerous chemical extraction or unsafe lithium-cell processing.

Recompile the TCZ extension from its source with:

```bash
python3 tools/compile_engineering_expert_module.py
```

## Memory-mapped weight loading

The kernel maps a weight file read-only and validates every requested tensor range before reading. It avoids loading the full model into heap memory; only the tensor slice requested by `TensorSpec` is materialized into a Candle `Tensor`.

```rust
use vortex_atoms_ai::{KernelConfig, TensorSpec, VortexAtomsKernel, WeightDType};

# fn load() -> Result<(), vortex_atoms_ai::VortexAtomsError> {
let kernel = VortexAtomsKernel::new(KernelConfig::default(), "weights.bin")?;
let spec = TensorSpec {
    name: "projection.weight".to_string(),
    offset_bytes: 0,
    dims: vec![768, 768],
    dtype: WeightDType::F32Le,
};
let tensor = kernel.load_tensor(&spec)?;
# Ok(())
# }
```

All names and identifiers in this project use the `vortex_atoms_ai` identity.

## Documentation

| Document | Description |
|----------|-------------|
| [User Guide](docs/USER_GUIDE.md) | End-user guide for the chat interface and dashboard |
| [API Reference](docs/API_REFERENCE.md) | Complete HTTP/WebSocket API documentation |
| [Architecture](docs/ARCHITECTURE.md) | System design and the 5-Kernel Matrix |
| [Developer Guide](docs/DEVELOPER_GUIDE.md) | Build, test, and contribution workflows |
| [Knowledge Modules](docs/knowledge/MODULES.md) | Domain knowledge fragments and how to add new ones |

## Testing

```bash
# Backend (Rust) — 155 tests (lib suite)
cargo test --features learner --lib

# Frontend (Vitest) — 354 tests
cd frontend && npm run test

# Frontend E2E (Playwright) — 54 tests
cd frontend && npm run test:e2e

# Frontend coverage report (threshold 70%)
cd frontend && npm run test:coverage
```

Current certified state (all green):

| Suite | Result |
|-------|--------|
| Rust `cargo test --features learner --lib` | 156 passed, 0 failed |
| Rust `cargo clippy --all-targets --features learner,tray -- -D warnings` | 0 errors |
| Frontend `vitest run` | 356/356 (22 files, incl. auth-client + markdown-XSS) |
| Frontend E2E `playwright test` | 54/54 (18 × Chromium/Firefox/WebKit: 51 clean + 3 Firefox flakes green on retry) |
| Frontend coverage | 99.49% statements · 99.4% branches · 99.42% functions · 99.88% lines |
| Frontend `oxlint` | 0 errors |
| Frontend `vite build` | success |

The only uncovered lines are intentionally unreachable guard branches and `lazy()` module
loaders, documented in `docs/DEVELOPER_GUIDE.md`.

## Community

- [Contributing Guide](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security Policy](SECURITY.md)
- [Investor Data Room](docs/data_room/00_INDEX.md)
- [Support](SUPPORT.md)
- [Changelog](CHANGELOG.md)

## License

Licensed under either of MIT or Apache-2.0 at your option. See [LICENSE](LICENSE).
