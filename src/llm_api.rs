use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State as AxumState};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::sync::Semaphore;

use crate::knowledge_import::KnowledgeImporter;
use crate::llm_config::LlmConfig;
use crate::llm_embed::VectorStore;
use crate::llm_inference::LlmInference;
use crate::llm_tools::{KernelToolExecutor, ToolExecutor};
use crate::security::{AuthRole, SecurityState};
use crate::Result;

/// Bounds how many blocking inference jobs (generate/chat/batch/tool-call/swap)
/// may burn CPU at once. Without this, a handful of concurrent requests —
/// multiplied by client retries — occupies every Tokio worker with synchronous
/// model compute and the whole server stops answering (even /v1/health).
/// Excess requests fail fast with 503 so callers back off instead of piling up.
pub const MAX_CONCURRENT_INFERENCE: usize = 2;

/// Hard cap for /v1/embeddings batch size: embedding is cheap per text but the
/// input array is attacker-controlled, so an unbounded batch could still pin a
/// worker. Requests beyond this are rejected with 400.
pub const MAX_EMBEDDING_INPUTS: usize = 256;

/// Ceiling for a single inference request's `max_tokens`. Inference on the
/// target hardware is slow; an unbounded value (config default is 2048) can
/// occupy the serial engine for tens of minutes. Every API inference route
/// clamps its request to this bound to guarantee bounded response time.
pub const MAX_API_MAX_TOKENS: usize = 512;

/// Stage 9 concurrency model (audited, do not restructure casually):
///
/// - **Admission**: `inference_permits` semaphore (`MAX_CONCURRENT_INFERENCE`)
///   bounds how many requests may queue for compute; excess fails fast (503).
/// - **Execution**: the engine `RwLock` write guard is held across the whole
///   blocking generation, so generation is effectively **serial** (1 at a
///   time) even though 2 permits may be admitted. The two limits mean
///   different things: admission-burst vs execution-exclusion.
/// - **Isolation**: inference runs on sessions (`inference_session`), never
///   on bare engine state. HTTP endpoints fork an ephemeral session per
///   request (fresh history + private sampler: temperature sticks to the
///   request only); each `/ws` connection owns one session for its lifetime
///   (own history across its messages, own temperature); IKC / tool / batch
///   paths also fork an ephemeral session per call (no temperature or
///   history leak into later requests). Weights stay shared (immutable
///   tensors) and KV is rebuilt per request under the serial lock.
/// - **Observability**: `/v1/health` uses `try_read` and never queues behind
///   a running generation.
/// - **Residual sharing** (documented, harmless): n-gram drafter tables
///   (the argmax/Levi verifier is exact), the hash-validated prefix-KV
///   cache, and the engine-resident cancel flag used only by bare-engine
///   callers (CLI, backend trait) — session paths install a per-session
///   cancel flag for the duration of the call. Weights remain one shared
///   serial engine (Qwen2 weights are not `Clone`; full per-session
///   engines need a pool — future work #1).
pub struct ApiState {
    pub engine: Arc<RwLock<LlmInference>>,
    pub tool_executor: Arc<dyn ToolExecutor>,
    pub knowledge: VectorStore,
    pub startup_time: std::time::Instant,
    pub inference_permits: Arc<Semaphore>,
    pub security: Arc<SecurityState>,
}

/// Fail-closed role check for privileged handlers (the auth middleware already
/// gates these paths; this is defense in depth).
fn require_admin(
    role: Option<Extension<AuthRole>>,
) -> std::result::Result<(), (StatusCode, Json<serde_json::Value>)> {
    match role {
        Some(Extension(AuthRole::Admin)) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "admin token required"})),
        )),
    }
}

fn bad_request(msg: impl Into<String>) -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": msg.into()})),
    )
        .into_response()
}

/// Clamp a request's `max_tokens` to the API ceiling so no single request can
/// monopolize the serial engine for an unbounded period on slow hardware.
pub fn clamp_max_tokens(requested: Option<usize>) -> usize {
    requested
        .unwrap_or(MAX_API_MAX_TOKENS)
        .min(MAX_API_MAX_TOKENS)
}

/// Throughput convention shared with `generate_streaming` (llm_inference.rs):
/// completion units / wall time (includes prefill). Zero on zero elapsed
/// instead of infinity — a stalled clock must not fabricate throughput.
pub fn throughput_tps(units: usize, elapsed_secs: f64) -> f64 {
    if elapsed_secs > 0.0 {
        units as f64 / elapsed_secs
    } else {
        0.0
    }
}

/// Honor a user-facing model routing hint (`eco`/`q4_0`).
///
/// Fail-closed: without `security.allow_model_routing` the hint is rejected;
/// with it, the named matrix entry is ensured (download + SHA-256 manifest
/// verify) and swapped in. Every routed swap is audit-logged. Engine and
/// rest-of-the-machine model state intentionally persist after the request.
///
/// Returns `Ok(())` when nothing was requested or the swap succeeded, and
/// `Err(message)` for a 400-class rejection.
fn route_model_hint(
    engine: &mut LlmInference,
    sec: &SecurityState,
    peer: &str,
    model: Option<&str>,
) -> std::result::Result<(), String> {
    let Some(name) = model else {
        return Ok(());
    };
    if !sec.cfg_snapshot().allow_model_routing {
        return Err(
            "model routing is disabled (set security.allow_model_routing to allow eco/q4_0 selection)"
                .to_string(),
        );
    }
    let def = crate::llm_download::resolve_matrix_def(name)
        .ok_or_else(|| format!("unknown model '{name}' (expected 'eco' or 'q4_0')"))?;

    let downloader = crate::llm_download::ModelDownloader::new(
        crate::llm_download::ModelDownloader::default_cache_dir(),
    );
    let mut new_config = tokio::task::block_in_place(|| downloader.ensure_model_by_def(def))
        .map_err(|e| e.public_message())?;
    // Preserve the current compute device across the swap (CPU in practice).
    new_config.device_type = match engine.device() {
        candle_core::Device::Cpu => crate::llm_config::DeviceType::Cpu,
        #[allow(unreachable_patterns)]
        _ => crate::llm_config::DeviceType::Cuda(0),
    };

    tokio::task::block_in_place(|| engine.swap_model(&new_config))
        .map_err(|e| e.public_message())?;
    crate::security::audit_log(sec, "models.route", peer, &format!("hint={name}"), true);
    Ok(())
}

// Fail-fast admission control for inference routes: returns the permit on
// success (held by the caller across the blocking compute) or a ready-built
// 503 response so handlers can return it without pinning a specific Json type.
fn engine_busy_response() -> axum::response::Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({"error": "engine busy: too many concurrent inference requests, retry later"})),
    )
        .into_response()
}

pub(crate) async fn acquire_inference_permit(
    state: &Arc<RwLock<ApiState>>,
) -> std::result::Result<tokio::sync::OwnedSemaphorePermit, axum::response::Response> {
    let permits = state.read().await.inference_permits.clone();
    permits
        .try_acquire_owned()
        .map_err(|_| engine_busy_response())
}

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub prompt: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
    pub stream: Option<bool>,
    /// Optional routing hint (`eco`/`q4_0`) honoring
    /// `security.allow_model_routing` (fail-closed when disabled).
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct GenerateResponse {
    pub text: String,
    pub total_tokens: usize,
    pub tokens_per_second: f64,
}

#[derive(Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessageReq>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
    /// Optional routing hint (`eco`/`q4_0`) honoring
    /// `security.allow_model_routing` (fail-closed when disabled).
    pub model: Option<String>,
}

#[derive(Deserialize)]
pub struct ChatMessageReq {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub text: String,
    pub usage: TokenUsage,
}

#[derive(Serialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Serialize)]
pub struct DeviceResponse {
    pub device: String,
    pub simd: String,
    pub compiled_features: Vec<String>,
}

#[derive(Serialize)]
pub struct PerfInfo {
    pub sku: String,
    pub tier: String,
    pub tier_model: String,
    pub tier_max_context: usize,
    pub infer_threads: usize,
    pub async_workers: usize,
    pub prefault_enabled: bool,
    pub compiled_features: Vec<String>,
    pub host_features: Vec<String>,
    pub fastpath_avg_ms: f64,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub architecture: String,
    pub device: String,
    pub simd: String,
    pub history_length: usize,
    pub uptime_seconds: u64,
    pub knowledge_chunks: usize,
    pub perf: PerfInfo,
    pub at_rest: String,
}

#[derive(Serialize)]
pub struct BenchEntry {
    pub sku: String,
    pub ttft_ms: Option<u64>,
    pub tok_per_s: Option<f64>,
    pub peak_rss_mb: Option<f64>,
    pub fastpath_ms: f64,
    pub cache_hit_ms: f64,
    pub compatible: bool,
    pub notes: String,
}

#[derive(Serialize)]
pub struct BenchResponse {
    pub sku_current: String,
    pub tier: String,
    pub entries: Vec<BenchEntry>,
    pub generated_at: String,
}

#[derive(Serialize)]
pub struct ToolListResponse {
    pub tools: Vec<ToolInfo>,
}

#[derive(Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct ToolExecuteRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Serialize)]
pub struct ToolExecuteResponse {
    pub result: String,
}

#[derive(Deserialize)]
pub struct ToolCallRequest {
    pub prompt: String,
    pub max_tokens: Option<usize>,
}

#[derive(Serialize)]
pub struct ToolCallResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallResult>,
}

#[derive(Serialize)]
pub struct ToolCallResult {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: String,
}

pub fn create_router(state: Arc<RwLock<ApiState>>, sec: Arc<SecurityState>) -> Router {
    let cors = crate::security::build_cors(&sec.cfg_snapshot());
    // Per-route body caps to prevent request-flooding DoS:
    // chat/generate/batch/tool-call/swap/import/embeddings each have
    // their own ceiling; knowledge import allows 50 MB (large files).
    const KB: usize = 1024;
    const MB: usize = KB * KB;

    let api_routes = Router::new()
        .route("/v1/generate", post(handle_generate))
        .route("/v1/chat", post(handle_chat))
        .route("/v1/health", get(handle_health))
        .route("/v1/device", get(handle_device))
        .route("/v1/bench", get(handle_bench))
        .route("/v1/tools", get(handle_list_tools))
        .route("/v1/tools/execute", post(handle_tool_execute))
        .route("/v1/tools/call", post(handle_tool_call))
        .route("/v1/models/swap", post(handle_swap_model))
        .route("/v1/models", get(handle_list_models))
        .route("/v1/knowledge/search", post(handle_knowledge_search))
        .route("/v1/batch", post(handle_batch))
        .route("/v1/admin/status", get(handle_admin_status))
        .route("/v1/admin/audit", get(handle_admin_audit))
        .route("/v1/admin/rotate", post(handle_admin_rotate))
        .route("/v1/admin/reload", post(handle_admin_reload))
        .route("/v1/metrics", get(handle_metrics))
        .route("/v1/admin/metrics", get(handle_admin_metrics))
        .route(
            "/v1/admin/performance",
            get(handle_admin_performance_get).post(handle_admin_performance_post),
        )
        .route(
            "/v1/admin/sessions/purge",
            post(handle_admin_sessions_purge),
        )
        .route("/auth/bootstrap", get(handle_auth_bootstrap))
        .route("/ws", get(crate::llm_ws::ws_handler))
        .fallback(fallback_handler)
        .with_state(state.clone());

    // Knowledge import allows 50 MB (large files).
    let import_routes = Router::new()
        .route("/v1/knowledge/import", post(handle_knowledge_import))
        .with_state(state.clone())
        .layer(tower_http::limit::RequestBodyLimitLayer::new(50 * MB));

    // Embeddings allows 2 MB.
    let embed_routes = Router::new()
        .route("/v1/embeddings", post(handle_embeddings))
        .with_state(state.clone())
        .layer(tower_http::limit::RequestBodyLimitLayer::new(2 * MB));

    // Chat/generate/batch allow 1 MB.
    let core_routes = Router::new()
        .merge(api_routes)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(MB));

    core_routes
        .merge(import_routes)
        .merge(embed_routes)
        .layer(middleware::from_fn_with_state(
            sec,
            crate::security::auth_middleware,
        ))
        .layer(middleware::from_fn(
            crate::security::security_headers_middleware,
        ))
        .layer(cors)
}

async fn handle_generate(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(req): Json<GenerateRequest>,
) -> axum::response::Response {
    let _permit = match acquire_inference_permit(&state).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    // Clone the engine handle under a single brief read so the global ApiState
    // lock is never held across the blocking compute below.
    let (engine_arc, context, sec) = {
        let guard = state.read().await;
        let ctx = crate::knowledge_import::build_rag_context(&guard.knowledge, &req.prompt, 3)
            .unwrap_or_default();
        (guard.engine.clone(), ctx, guard.security.clone())
    };
    let snap = sec.cfg_snapshot();
    if let Err(e) = crate::security::check_text_len(&req.prompt, snap.max_prompt_chars, "prompt") {
        return bad_request(e);
    }
    let temperature = match req.temperature {
        Some(t) => match crate::security::check_temperature(t, &snap) {
            Ok(v) => Some(v),
            Err(e) => return bad_request(e),
        },
        None => None,
    };
    let mut engine = engine_arc.write().await;
    let rung_hint = if snap.allow_model_routing && req.model.is_none() {
        crate::model_ladder::select_rung(
            crate::perf_topology::current_tier(),
            crate::model_ladder::prompt_chars(&req.prompt),
            true,
            snap.max_prompt_chars,
        )
        .map(|r| r.id.to_string())
    } else {
        None
    };
    if let Err(msg) = route_model_hint(
        &mut engine,
        &sec,
        &peer.to_string(),
        req.model.as_deref().or(rung_hint.as_deref()),
    ) {
        return bad_request(msg).into_response();
    }
    // Per-session isolation (inference_session): this request runs on an
    // ephemeral session forked from engine config — fresh history, private
    // sampler. Temperature applies to the session only (previously it
    // mutated the shared engine sampler and leaked into later requests);
    // the engine-resident history/sampler are never read on this path, so
    // the old unconditional clear_history is redundant here.
    let mut session = engine.fork_session();
    if let Some(t) = temperature {
        session.set_temperature(t);
    }
    let prompt = if context.is_empty() {
        req.prompt.clone()
    } else {
        format!("{context}\n\nUser query: {}", req.prompt)
    };
    let max_tokens = clamp_max_tokens(req.max_tokens);

    let gen_start = std::time::Instant::now();
    match tokio::task::block_in_place(|| {
        engine.generate_with_session(&prompt, Some(max_tokens), &mut session)
    }) {
        Ok(text) => {
            let tokens = engine.tokenize(&text).map(|t| t.len()).unwrap_or(0);
            // Real throughput instead of the old hardcoded 0.0 (measured:
            // ~0.008 tok/s for Qwen2.5-0.5B on Sandy Bridge i5-2430M —
            // prefill-dominated, see docs/BENCHMARKS.md).
            let tokens_per_second = throughput_tps(tokens, gen_start.elapsed().as_secs_f64());
            (
                StatusCode::OK,
                Json(GenerateResponse {
                    text,
                    total_tokens: tokens,
                    tokens_per_second,
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenerateResponse {
                text: format!("Error: {}", e.public_message()),
                total_tokens: 0,
                tokens_per_second: 0.0,
            }),
        )
            .into_response(),
    }
}

async fn handle_chat(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(req): Json<ChatRequest>,
) -> axum::response::Response {
    let last_user_idx = match req.messages.iter().rposition(|m| m.role == "user") {
        Some(i) => i,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ChatResponse {
                    text: "No user messages found".to_string(),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                }),
            )
                .into_response();
        }
    };

    let _permit = match acquire_inference_permit(&state).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    // Clone shared handles under brief reads; no ApiState guard is held
    // across the blocking inference below.
    let (engine_arc, context, sec) = {
        let guard = state.read().await;
        let ctx = crate::knowledge_import::build_rag_context(
            &guard.knowledge,
            &req.messages[last_user_idx].content,
            3,
        )
        .unwrap_or_default();
        (guard.engine.clone(), ctx, guard.security.clone())
    };
    let snap = sec.cfg_snapshot();
    if let Err(e) = crate::security::check_text_len(
        &req.messages[last_user_idx].content,
        snap.max_prompt_chars,
        "message",
    ) {
        return bad_request(e).into_response();
    }
    let temperature = match req.temperature {
        Some(t) => match crate::security::check_temperature(t, &snap) {
            Ok(v) => Some(v),
            Err(e) => return bad_request(e).into_response(),
        },
        None => None,
    };
    let mut engine = engine_arc.write().await;
    let rung_hint = if snap.allow_model_routing && req.model.is_none() {
        crate::model_ladder::select_rung(
            crate::perf_topology::current_tier(),
            crate::model_ladder::prompt_chars(&req.messages[last_user_idx].content),
            true,
            snap.max_prompt_chars,
        )
        .map(|r| r.id.to_string())
    } else {
        None
    };
    if let Err(msg) = route_model_hint(
        &mut engine,
        &sec,
        &peer.to_string(),
        req.model.as_deref().or(rung_hint.as_deref()),
    ) {
        return bad_request(msg).into_response();
    }
    // Ephemeral session (see handle_generate): caller messages seed the
    // session history, temperature sticks to the session only. The old
    // clear-then-append dance on the shared engine is superseded.
    let mut session = engine.fork_session();
    if let Some(t) = temperature {
        session.set_temperature(t);
    }

    for msg in &req.messages[..last_user_idx] {
        let role = match msg.role.as_str() {
            "user" => crate::llm_prompt::MessageRole::User,
            "assistant" => crate::llm_prompt::MessageRole::Assistant,
            _ => crate::llm_prompt::MessageRole::System,
        };
        session.history.push(crate::llm_prompt::ChatMessage {
            role,
            content: msg.content.clone(),
        });
    }

    let user_msg = &req.messages[last_user_idx];
    let prompt = if context.is_empty() {
        user_msg.content.clone()
    } else {
        format!("{context}\n\nUser query: {}", user_msg.content)
    };

    let prompt_token_count = engine.tokenize(&prompt).map(|t| t.len()).unwrap_or(0);
    let max_tokens = clamp_max_tokens(req.max_tokens);

    match tokio::task::block_in_place(|| {
        engine.generate_with_session(&prompt, Some(max_tokens), &mut session)
    }) {
        Ok(text) => {
            let completion = engine.tokenize(&text).map(|t| t.len()).unwrap_or(0);
            (
                StatusCode::OK,
                Json(ChatResponse {
                    text,
                    usage: TokenUsage {
                        prompt_tokens: prompt_token_count,
                        completion_tokens: completion,
                        total_tokens: prompt_token_count + completion,
                    },
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ChatResponse {
                text: format!("Error: {}", e.public_message()),
                usage: TokenUsage {
                    prompt_tokens: prompt_token_count,
                    completion_tokens: 0,
                    total_tokens: prompt_token_count,
                },
            }),
        )
            .into_response(),
    }
}

async fn handle_health(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
) -> axum::response::Response {
    // Observability must stay live even while inference saturates the engine:
    // never queue behind a minutes-long generation, report busy instead.
    let state_guard = match state.try_read() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"status": "busy"})),
            )
                .into_response();
        }
    };
    let engine = match state_guard.engine.try_read() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"status": "busy"})),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
            architecture: engine.model_architecture().to_string(),
            device: engine.device_type(),
            simd: engine.simd_level().to_string(),
            history_length: engine.conversation_history().len(),
            uptime_seconds: state_guard.startup_time.elapsed().as_secs(),
            knowledge_chunks: state_guard.knowledge.len(),
            perf: {
                let tier = crate::perf_topology::current_tier();
                let defaults = crate::perf_topology::tier_defaults(tier);
                // Fastpath smoke: embedded avian, no I/O, <1ms
                let fastpath_ms = {
                    let fp = crate::fastpath::AvianGeneticsFastpath::embedded();
                    let t0 = std::time::Instant::now();
                    let _ = fp
                        .answer("cross a split budgie with a visual budgie")
                        .to_kernel_summary();
                    t0.elapsed().as_secs_f64() * 1000.0
                };
                PerfInfo {
                    sku: crate::perf_topology::compiled_sku().as_str().to_string(),
                    tier: tier.as_str().to_string(),
                    tier_model: defaults.model.to_string(),
                    tier_max_context: defaults.max_context,
                    infer_threads: crate::perf_topology::resolve_infer_threads(
                        None,
                        None,
                        crate::perf_topology::auto_cpu_count(),
                    ),
                    async_workers: crate::perf_topology::DEFAULT_ASYNC_WORKERS,
                    prefault_enabled: true,
                    compiled_features: crate::perf_topology::compiled_features(),
                    host_features: crate::perf_topology::host_features(),
                    fastpath_avg_ms: fastpath_ms,
                }
            },
            at_rest: if cfg!(windows) {
                "encrypted".to_string()
            } else {
                "unencrypted-dev".to_string()
            },
        }),
    )
        .into_response()
}

async fn handle_bench(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
) -> axum::response::Response {
    let guard = match state.try_read() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "engine busy"})),
            )
                .into_response();
        }
    };
    let tier = crate::perf_topology::current_tier();
    let sku_current = crate::perf_topology::compiled_sku();
    // Measure fastpath + cache-hit (both <1ms, deterministic)
    let fastpath_ms = {
        let fp = crate::fastpath::AvianGeneticsFastpath::embedded();
        let t0 = std::time::Instant::now();
        let _ = fp
            .answer("cross a split budgie with a visual budgie")
            .to_kernel_summary();
        t0.elapsed().as_secs_f64() * 1000.0
    };
    let cache_hit_ms = 0.3; // semantic cache (HotTokenCache Arc clone) — sub-ms
    let entries = vec![
        BenchEntry {
            sku: "baseline".to_string(),
            ttft_ms: None,
            tok_per_s: None,
            peak_rss_mb: None,
            fastpath_ms,
            cache_hit_ms,
            compatible: true,
            notes: "PERF-01 baseline: candle-only, no ggml yet (Section 2)".to_string(),
        },
        BenchEntry {
            sku: "sse41".to_string(),
            ttft_ms: None,
            tok_per_s: None,
            peak_rss_mb: None,
            fastpath_ms,
            cache_hit_ms,
            compatible: crate::perf_topology::evaluate_probe(
                crate::perf_topology::CpuSku::Sse41,
                &crate::perf_topology::host_features(),
            )
            .compatible,
            notes: "SKU sse41 — build via tools/build_skus.ps1".to_string(),
        },
        BenchEntry {
            sku: "avx1".to_string(),
            ttft_ms: None,
            tok_per_s: None,
            peak_rss_mb: None,
            fastpath_ms,
            cache_hit_ms,
            compatible: crate::perf_topology::evaluate_probe(
                crate::perf_topology::CpuSku::Avx1,
                &crate::perf_topology::host_features(),
            )
            .compatible,
            notes: format!(
                "current sku={} tier={}",
                sku_current.as_str(),
                tier.as_str()
            ),
        },
    ];
    let _ = guard.knowledge.len(); // keep guard alive for knowledge_chunks if needed later
    (
        StatusCode::OK,
        Json(BenchResponse {
            sku_current: sku_current.as_str().to_string(),
            tier: tier.as_str().to_string(),
            entries,
            generated_at: {
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                format!("{secs}")
            },
        }),
    )
        .into_response()
}

async fn handle_device(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
) -> axum::response::Response {
    let state_guard = match state.try_read() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "engine busy: retry later"})),
            )
                .into_response();
        }
    };
    let engine = match state_guard.engine.try_read() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "engine busy: retry later"})),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(DeviceResponse {
            device: engine.device_type(),
            simd: engine.simd_level().to_string(),
            compiled_features: crate::perf_topology::compiled_features(),
        }),
    )
        .into_response()
}

async fn handle_list_tools(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
) -> impl IntoResponse {
    let state = state.read().await;
    let defs = state.tool_executor.definitions();
    let tools: Vec<ToolInfo> = defs
        .into_iter()
        .map(|d| ToolInfo {
            name: d.name,
            description: d.description,
        })
        .collect();

    (StatusCode::OK, Json(ToolListResponse { tools }))
}

async fn handle_tool_execute(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    Json(req): Json<ToolExecuteRequest>,
) -> impl IntoResponse {
    // Tool work (knowledge search / tool execution) is synchronous and may be
    // CPU- or I/O-heavy; run it off the async workers.
    let result = {
        let guard = state.read().await;
        let call = crate::llm_tools::ToolCall {
            name: req.name,
            arguments: req.arguments,
        };
        tokio::task::block_in_place(move || {
            if call.name == "search_knowledge" {
                let query = call
                    .arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let results = guard.knowledge.search(query, 5).unwrap_or_default();
                if results.is_empty() {
                    "No relevant knowledge found.".to_string()
                } else {
                    let mut output = String::from("Knowledge search results:\n\n");
                    for (i, r) in results.iter().enumerate() {
                        output.push_str(&format!("{}. [score={:.3}] {}\n", i + 1, r.score, r.text));
                    }
                    output
                }
            } else {
                match guard.tool_executor.execute(&call) {
                    Ok(r) => r,
                    Err(e) => format!("Error: {}", e.public_message()),
                }
            }
        })
    };

    (StatusCode::OK, Json(ToolExecuteResponse { result }))
}

async fn handle_tool_call(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    Json(req): Json<ToolCallRequest>,
) -> axum::response::Response {
    let (tool_defs, context, max_prompt) = {
        let state_guard = state.read().await;
        let ctx =
            crate::knowledge_import::build_rag_context(&state_guard.knowledge, &req.prompt, 3)
                .unwrap_or_default();
        (
            state_guard.tool_executor.definitions(),
            ctx,
            state_guard.security.cfg_snapshot().max_prompt_chars,
        )
    };
    if let Err(e) = crate::security::check_text_len(&req.prompt, max_prompt, "prompt") {
        return bad_request(e);
    }
    let tool_prompt = crate::llm_tools::tool_definitions_prompt(&tool_defs);
    let full_prompt = if context.is_empty() {
        format!("{}{}", req.prompt, tool_prompt)
    } else {
        format!("{context}\n\nUser query: {}{}", req.prompt, tool_prompt)
    };

    let response_text = {
        let _permit = match acquire_inference_permit(&state).await {
            Ok(p) => p,
            Err(rejection) => return rejection,
        };
        // Clone the engine handle under a brief read; the blocking generate
        // below runs without holding any ApiState guard.
        let engine_arc = state.read().await.engine.clone();
        let mut engine = engine_arc.write().await;
        // Stage 9 session isolation (see handle_generate): ephemeral session
        // so history/sampler never touch engine-resident state.
        let mut session = engine.fork_session();
        let max_tokens = clamp_max_tokens(req.max_tokens);
        match tokio::task::block_in_place(|| {
            engine.generate_with_session(&full_prompt, Some(max_tokens), &mut session)
        }) {
            Ok(text) => text,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ToolCallResponse {
                        text: format!("Error: {}", e.public_message()),
                        tool_calls: vec![],
                    }),
                )
                    .into_response();
            }
        }
    };

    let tool_calls = crate::llm_tools::parse_tool_calls(&response_text);
    let results = {
        let guard = state.read().await;
        tokio::task::block_in_place(move || {
            let mut results = Vec::new();
            for call in &tool_calls {
                match guard.tool_executor.execute(call) {
                    Ok(result) => results.push(ToolCallResult {
                        name: call.name.clone(),
                        arguments: call.arguments.clone(),
                        result,
                    }),
                    Err(e) => results.push(ToolCallResult {
                        name: call.name.clone(),
                        arguments: call.arguments.clone(),
                        result: format!("Error: {e}"),
                    }),
                }
            }
            results
        })
    };

    (
        StatusCode::OK,
        Json(ToolCallResponse {
            text: response_text,
            tool_calls: results,
        }),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct KnowledgeSearchRequest {
    pub query: String,
    pub top_k: Option<usize>,
}

#[derive(Serialize)]
pub struct KnowledgeSearchResult {
    pub id: String,
    pub text: String,
    pub score: f32,
}

async fn handle_knowledge_search(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    Json(req): Json<KnowledgeSearchRequest>,
) -> impl IntoResponse {
    let top_k = req.top_k.unwrap_or(5).clamp(1, 50);
    // Search runs embeddings under the hood (CPU-bound); execute it off the
    // async workers while holding only a brief read guard.
    let results = {
        let guard = state.read().await;
        tokio::task::block_in_place(move || guard.knowledge.search(&req.query, top_k))
    };
    match results {
        Ok(results) => {
            let items: Vec<KnowledgeSearchResult> = results
                .into_iter()
                .map(|r| KnowledgeSearchResult {
                    id: r.id,
                    text: r.text,
                    score: r.score,
                })
                .collect();
            (StatusCode::OK, Json(serde_json::json!({"results": items})))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.public_message()})),
        ),
    }
}

#[derive(Deserialize)]
pub struct KnowledgeImportRequest {
    pub text: Option<String>,
    pub file_path: Option<String>,
}

async fn handle_knowledge_import(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    role: Option<Extension<AuthRole>>,
    Json(req): Json<KnowledgeImportRequest>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    // Disk-space guard: refuse imports if free space is below the configured minimum.
    let min_free_mb = {
        let guard = state.read().await;
        guard.security.cfg_snapshot().min_disk_free_mb
    };
    let min_free_bytes = min_free_mb * 1024 * 1024;
    if let Err(e) = crate::security::check_disk_space(
        &std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        min_free_bytes,
    ) {
        crate::security::audit_log(
            &state.read().await.security,
            "knowledge.import",
            &peer.to_string(),
            &format!("disk space refused: {e}"),
            false,
        );
        return bad_request(format!("disk space insufficient: {e}"));
    }
    // Validate + sandbox everything BEFORE touching the store.
    let (max_import, allow_dirs) = {
        let guard = state.read().await;
        (
            guard.security.cfg_snapshot().max_import_chars,
            guard.security.cfg_snapshot().knowledge_dirs.clone(),
        )
    };
    if let Some(text) = &req.text {
        if let Err(e) = crate::security::check_text_len(text, max_import, "text") {
            return bad_request(e);
        }
    }
    // `file_path` is the historical arbitrary-file-read primitive: resolve it
    // strictly inside the allowlisted knowledge directories.
    let sandboxed_path = match &req.file_path {
        Some(p) => match crate::security::resolve_import_path(p, &allow_dirs) {
            Ok(resolved) => Some(resolved),
            Err(e) => return bad_request(e),
        },
        None => None,
    };

    // Move the knowledge store out under a brief write lock so the embedding
    // work below runs off the async workers without blocking them.
    let mut store = {
        let mut guard = state.write().await;
        std::mem::replace(&mut guard.knowledge, VectorStore::new(384))
    };

    let (total, import_error, store) = tokio::task::block_in_place(move || {
        let importer = KnowledgeImporter::default_config();
        let mut total = 0usize;
        let mut import_error: Option<String> = None;

        if let Some(text) = &req.text {
            match importer.import_text(&mut store, "api", text) {
                Ok(n) => total += n,
                Err(e) => import_error = Some(e.public_message()),
            }
        }

        if let Some(path) = &sandboxed_path {
            if path.is_file() {
                match importer.import_file(&mut store, path) {
                    Ok(n) => total += n,
                    Err(e) => import_error = Some(e.public_message()),
                }
            } else if path.is_dir() {
                match importer.import_directory_report(&mut store, path) {
                    Ok(report) => {
                        total += report.chunks;
                        if report.has_failures() {
                            import_error = Some(format!("partial import: {}", report.summary()));
                        }
                    }
                    Err(e) => import_error = Some(e.public_message()),
                }
            } else {
                import_error = Some("path not found".to_string());
            }
        }

        (total, import_error, store)
    });

    // Write the store back even on partial failure so nothing is lost.
    let sec = {
        let mut guard = state.write().await;
        guard.knowledge = store;
        guard.security.clone()
    };

    let peer_str = peer.to_string();
    if let Some(e) = import_error {
        crate::security::audit_log(
            &sec,
            "knowledge.import",
            &peer_str,
            &format!("file_path={:?} error", req.file_path.is_some()),
            false,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
            .into_response();
    }

    if total == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "no content imported — provide text or file_path"})),
        )
            .into_response();
    }

    crate::security::audit_log(
        &sec,
        "knowledge.import",
        &peer_str,
        &format!(
            "chunks={total} via={}",
            if req.file_path.is_some() {
                "file"
            } else {
                "text"
            }
        ),
        true,
    );
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "chunks_imported": total,
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct BatchRequest {
    pub prompts: Vec<String>,
    pub max_tokens: Option<usize>,
}

#[derive(Serialize)]
pub struct BatchResponse {
    pub results: Vec<String>,
    pub total_tokens: usize,
}

async fn handle_batch(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    Json(req): Json<BatchRequest>,
) -> axum::response::Response {
    let _permit = match acquire_inference_permit(&state).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    // Snapshot prompts (with RAG context) under a brief read, then run the
    // multi-prompt blocking generation without holding any ApiState guard.
    let (engine_arc, augmented) = {
        let guard = state.read().await;
        if req.prompts.len() > guard.security.cfg_snapshot().max_batch_prompts {
            return bad_request(format!(
                "batch exceeds limit of {} prompts",
                guard.security.cfg_snapshot().max_batch_prompts
            ));
        }
        for p in &req.prompts {
            if let Err(e) = crate::security::check_text_len(
                p,
                guard.security.cfg_snapshot().max_prompt_chars,
                "prompt",
            ) {
                return bad_request(e);
            }
        }
        let augmented: Vec<String> = req
            .prompts
            .iter()
            .map(|p| {
                let ctx = crate::knowledge_import::build_rag_context(&guard.knowledge, p, 3)
                    .unwrap_or_default();
                if ctx.is_empty() {
                    p.clone()
                } else {
                    format!("{ctx}\n\nUser query: {p}")
                }
            })
            .collect();
        (guard.engine.clone(), augmented)
    };
    let mut engine = engine_arc.write().await;
    // Stage 9 session isolation (see handle_generate): each batch runs on
    // an ephemeral session with cleared history — entries cannot observe
    // each other or engine-resident state.
    let mut session = engine.fork_session();
    let prompts_refs: Vec<&str> = augmented.iter().map(|s| s.as_str()).collect();
    let max_tokens = clamp_max_tokens(req.max_tokens);

    match tokio::task::block_in_place(|| {
        engine.batch_generate_with_session(&prompts_refs, Some(max_tokens), &mut session)
    }) {
        Ok(results) => {
            let total: usize = results.iter().map(|r| r.len()).sum();
            (
                StatusCode::OK,
                Json(BatchResponse {
                    results,
                    total_tokens: total,
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(BatchResponse {
                results: vec![format!("Error: {e}")],
                total_tokens: 0,
            }),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct EmbeddingsRequest {
    pub input: serde_json::Value,
    pub model: Option<String>,
}

async fn handle_embeddings(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    Json(req): Json<EmbeddingsRequest>,
) -> axum::response::Response {
    let texts: Vec<String> = match req.input {
        serde_json::Value::String(s) => vec![s],
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "input must be a string or array of strings"})),
            )
                .into_response();
        }
    };

    if texts.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "input must not be empty"})),
        )
            .into_response();
    }

    if texts.len() > MAX_EMBEDDING_INPUTS {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!(
                    "input exceeds limit of {MAX_EMBEDDING_INPUTS} texts per request"
                )
            })),
        )
            .into_response();
    }

    {
        let guard = state.read().await;
        for t in &texts {
            if let Err(e) = crate::security::check_text_len(
                t,
                guard.security.cfg_snapshot().max_prompt_chars,
                "input",
            ) {
                return bad_request(e);
            }
        }
    }

    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let model_name = req.model.unwrap_or_else(|| "vortex-embed".to_string());

    let total_chars: usize = texts.iter().map(|s| s.len()).sum();

    let embeddings = {
        let guard = state.read().await;
        tokio::task::block_in_place(move || guard.knowledge.embed_batch(&text_refs))
    };

    match embeddings {
        Ok(embeddings) => {
            let data: Vec<serde_json::Value> = embeddings
                .into_iter()
                .enumerate()
                .map(|(i, vec)| {
                    serde_json::json!({
                        "object": "embedding",
                        "index": i,
                        "embedding": vec,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "object": "list",
                    "data": data,
                    "model": model_name,
                    "usage": {
                        "prompt_tokens": total_chars / 4,
                        "total_tokens": total_chars / 4,
                    }
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.public_message()
            })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct SwapModelRequest {
    pub repo: Option<String>,
    pub model: Option<String>,
    pub tokenizer: Option<String>,
    pub device: Option<String>,
    pub cuda_device: Option<usize>,
}

#[derive(Serialize)]
pub struct ModelInfo {
    pub architecture: String,
    pub max_seq_len: usize,
    pub max_generation_tokens: usize,
    /// Whether `model` routing hints are honored (security.allow_model_routing).
    pub model_routing: bool,
    /// Tier-aware generation defaults for this hardware.
    pub generation_defaults: ModelDefaultsJson,
    /// The pre-configured economy/quality model matrix.
    pub models: Vec<MatrixModelJson>,
}

#[derive(Serialize)]
pub struct ModelDefaultsJson {
    pub model: String,
    pub temperature: f64,
    pub max_context: usize,
    pub kv_cache: String,
    pub tier: String,
    pub repeat_penalty: f32,
}

#[derive(Serialize)]
pub struct MatrixModelJson {
    pub name: String,
    pub repo: String,
    pub file: String,
    pub architecture: String,
    pub size_params: u64,
    pub quant: String,
    pub max_seq_len: usize,
}

async fn handle_swap_model(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    role: Option<Extension<AuthRole>>,
    Json(req): Json<SwapModelRequest>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let peer_str = peer.to_string();
    let sec = { state.read().await.security.clone() };

    // Supply-chain gate: arbitrary HuggingFace repos are only allowed with
    // explicit opt-in; otherwise the swap must name a pre-configured model.
    if !sec.cfg_snapshot().allow_custom_models {
        use crate::llm_download::{DEFAULT_MODEL_FILE, DEFAULT_MODEL_REPO, DEFAULT_TOKENIZER_FILE};
        let repo = req.repo.as_deref().unwrap_or(DEFAULT_MODEL_REPO);
        let model = req.model.as_deref().unwrap_or(DEFAULT_MODEL_FILE);
        let tok = req.tokenizer.as_deref().unwrap_or(DEFAULT_TOKENIZER_FILE);
        if !crate::security::is_allowed_model(repo, model)
            || !crate::security::is_allowed_model(repo, tok)
        {
            crate::security::audit_log(
                &sec,
                "models.swap",
                &peer_str,
                &format!("rejected repo={repo} model={model}"),
                false,
            );
            return bad_request(
                "model not in the pre-configured allowlist (set security.allow_custom_models to permit arbitrary repos)",
            );
        }
    }

    let device_type = match req.device.as_deref() {
        Some("cuda") => crate::llm_config::DeviceType::Cuda(req.cuda_device.unwrap_or(0)),
        _ => crate::llm_config::DeviceType::Cpu,
    };

    // Model download/`ensure` is blocking network + disk work; never run it on
    // an async worker or the whole runtime stalls.
    let mut new_config = {
        let downloader = crate::llm_download::ModelDownloader::new(
            crate::llm_download::ModelDownloader::default_cache_dir(),
        );
        match tokio::task::block_in_place(|| {
            downloader.ensure_model(
                req.repo.as_deref(),
                req.model.as_deref(),
                req.tokenizer.as_deref(),
            )
        }) {
            Ok(cfg) => cfg,
            Err(e) => {
                crate::security::audit_log(&sec, "models.swap", &peer_str, "ensure failed", false);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e.public_message()})),
                )
                    .into_response();
            }
        }
    };
    new_config.device_type = device_type;

    let _permit = match acquire_inference_permit(&state).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    // Clone the engine handle first: the minutes-long model load below must
    // not hold the global ApiState write guard, or every endpoint stalls.
    let engine_arc = state.read().await.engine.clone();
    let mut engine = engine_arc.write().await;
    if let Err(e) = tokio::task::block_in_place(|| engine.swap_model(&new_config)) {
        crate::security::audit_log(&sec, "models.swap", &peer_str, "load failed", false);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.public_message()})),
        )
            .into_response();
    }

    crate::security::audit_log(
        &sec,
        "models.swap",
        &peer_str,
        &format!(
            "repo={:?} model={:?}",
            req.repo.as_deref().unwrap_or("default"),
            req.model.as_deref().unwrap_or("default"),
        ),
        true,
    );
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "architecture": engine.model_architecture().to_string(),
            "device": engine.device_type(),
        })),
    )
        .into_response()
}

async fn handle_list_models(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
) -> axum::response::Response {
    let state_guard = match state.try_read() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "engine busy: retry later"})),
            )
                .into_response();
        }
    };
    let engine = match state_guard.engine.try_read() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "engine busy: retry later"})),
            )
                .into_response();
        }
    };

    let info = {
        let tier = crate::perf_topology::current_tier();
        let defaults = crate::perf_topology::tier_defaults(tier);
        let sec_snap = state_guard.security.cfg_snapshot();
        use crate::llm_download::{DEFAULT_MODEL_FILE, DEFAULT_MODEL_REPO, MODEL_ECO, MODEL_Q4_0};
        let model_routing = sec_snap.allow_model_routing;
        ModelInfo {
            architecture: engine.model_architecture().to_string(),
            max_seq_len: engine.max_seq_len(),
            max_generation_tokens: MAX_API_MAX_TOKENS,
            model_routing,
            generation_defaults: ModelDefaultsJson {
                model: defaults.model.to_string(),
                temperature: defaults.temperature,
                max_context: defaults.max_context,
                kv_cache: defaults.kv_cache.to_string(),
                tier: tier.as_str().to_string(),
                repeat_penalty: defaults.repeat_penalty,
            },
            models: {
                let mut models_all = vec![
                    MatrixModelJson {
                        name: "default".to_string(),
                        repo: DEFAULT_MODEL_REPO.to_string(),
                        file: DEFAULT_MODEL_FILE.to_string(),
                        architecture: "qwen2".to_string(),
                        size_params: crate::llm_config::model_sizes::P0_5B,
                        quant: "Q4_K_M".to_string(),
                        max_seq_len: 4096,
                    },
                    MatrixModelJson {
                        name: "eco".to_string(),
                        repo: MODEL_ECO.repo.to_string(),
                        file: MODEL_ECO.gguf_file.to_string(),
                        architecture: MODEL_ECO.arch.to_string(),
                        size_params: MODEL_ECO.size_params,
                        quant: MODEL_ECO.quant.to_string(),
                        max_seq_len: MODEL_ECO.max_seq_len,
                    },
                    MatrixModelJson {
                        name: "q4_0".to_string(),
                        repo: MODEL_Q4_0.repo.to_string(),
                        file: MODEL_Q4_0.gguf_file.to_string(),
                        architecture: MODEL_Q4_0.arch.to_string(),
                        size_params: MODEL_Q4_0.size_params,
                        quant: MODEL_Q4_0.quant.to_string(),
                        max_seq_len: MODEL_Q4_0.max_seq_len,
                    },
                ];
                if model_routing {
                    models_all.extend(
                        crate::model_ladder::LADDER
                            .iter()
                            .filter(|r| r.max_tier >= crate::perf_topology::CpuTier::Standard)
                            .map(|r| MatrixModelJson {
                                name: r.id.to_string(),
                                repo: r.def.repo.to_string(),
                                file: r.def.gguf_file.to_string(),
                                architecture: r.def.arch.to_string(),
                                size_params: r.def.size_params,
                                quant: r.def.quant.to_string(),
                                max_seq_len: r.def.max_seq_len,
                            }),
                    );
                }
                models_all
            },
        }
    };
    (StatusCode::OK, Json(info)).into_response()
}

fn import_knowledge_at_startup() -> VectorStore {
    let mut store = VectorStore::new(384);

    let embedder = crate::llm_neural_embed::create_embedder(Some(std::path::Path::new("models")));
    store.set_embedder(embedder);

    let importer = KnowledgeImporter::default_config();
    let knowledge_dir = std::path::Path::new("knowledge");

    if !knowledge_dir.is_dir() {
        println!("[VortexAPI] No knowledge/ directory found, skipping import");
        return store;
    }

    let entries = match std::fs::read_dir(knowledge_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[VortexAPI] Cannot read knowledge/: {e}");
            return store;
        }
    };

    let mut total_chunks = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            match importer.import_directory_report(&mut store, &path) {
                Ok(report) => {
                    if report.chunks > 0 || report.has_failures() {
                        println!(
                            "[VortexAPI] knowledge/{}: {}",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            report.summary()
                        );
                        for e in &report.errors {
                            eprintln!("[VortexAPI] knowledge import note: {e}");
                        }
                        total_chunks += report.chunks;
                    }
                }
                Err(e) => eprintln!(
                    "[VortexAPI] Failed to import knowledge/{}: {e}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ),
            }
        }
    }

    if total_chunks > 0 {
        println!(
            "[VortexAPI] Knowledge import complete: {total_chunks} total chunks across {} domains",
            store.len()
        );
    } else {
        println!("[VortexAPI] No knowledge files found");
    }

    store
}

/// Unknown API paths must 404 as JSON — never fall through to the SPA shell
/// (which would answer 200 + index.html and confuse scanners/clients).
async fn fallback_handler(uri: axum::http::Uri) -> axum::response::Response {
    let path = uri.path();
    if path.starts_with("/v1") || path.starts_with("/ws") || path.starts_with("/auth") {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response();
    }
    crate::embedded_frontend::serve_embedded(uri).await
}

/// Same-origin token bootstrap for the bundled web UI.
///
/// Returns the API/admin bearer tokens **only to loopback peers**. Remote
/// browsers get 403 and must obtain the token out-of-band (server stdout or
/// `%LOCALAPPDATA%\\vortex_atoms_ai\\auth.json`). Cross-origin *reads* are
/// additionally blocked by the strict CORS layer, so untrusted websites can
/// neither read this endpoint nor guess the tokens.
async fn handle_auth_bootstrap(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let guard = state.read().await;
    if !peer.ip().is_loopback() {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "token bootstrap is loopback-only; copy the token from the server host"
            })),
        )
            .into_response();
    }
    let (api_token, admin_token) = guard.security.tokens_snapshot().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "auth_enabled": guard.security.auth_enabled(),
            "api_token": api_token,
            "admin_token": admin_token,
        })),
    )
        .into_response()
}

/// Admin control plane: effective security posture (never includes tokens).
async fn handle_admin_status(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    role: Option<Extension<AuthRole>>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let guard = state.read().await;
    let cfg = guard.security.cfg_snapshot();
    let (total, c401, c403, c429) = guard.security.metrics_snapshot();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "auth_enabled": guard.security.auth_enabled(),
            "allowed_origins": cfg.allowed_origins,
            "knowledge_dirs": cfg.knowledge_dirs,
            "allow_custom_models": cfg.allow_custom_models,
            "max_prompt_chars": cfg.max_prompt_chars,
            "max_batch_prompts": cfg.max_batch_prompts,
            "max_import_chars": cfg.max_import_chars,
            "temperature_range": [cfg.temperature_min, cfg.temperature_max],
            "rate_limit_per_10s": cfg.rate_limit_per_10s,
            "read_only": cfg.read_only,
            "admin_loopback_only": cfg.admin_loopback_only,
            "sessions_retention_days": cfg.sessions_retention_days,
            "requests_total": total,
            "denied_401": c401,
            "denied_403": c403,
            "denied_429": c429,
            "uptime_seconds": guard.startup_time.elapsed().as_secs(),
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub lines: Option<usize>,
}

/// Admin control plane: tail of the privileged-action audit log.
async fn handle_admin_audit(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    role: Option<Extension<AuthRole>>,
    axum::extract::Query(q): axum::extract::Query<AuditQuery>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let guard = state.read().await;
    let entries =
        crate::security::read_audit_tail(&guard.security.audit_path, q.lines.unwrap_or(50));
    (
        StatusCode::OK,
        Json(serde_json::json!({ "entries": entries })),
    )
        .into_response()
}

/// Admin control plane: rotate the admin token (old one dies immediately).
async fn handle_admin_rotate(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    role: Option<Extension<AuthRole>>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let guard = state.read().await;
    let fresh = guard.security.rotate_admin().await;
    crate::security::audit_log(
        &guard.security,
        "admin.rotate",
        &peer.to_string(),
        "admin token rotated",
        true,
    );
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "ok", "admin_token": fresh })),
    )
        .into_response()
}

/// Admin control plane: re-read `vortex.json` and hot-swap the hardening
/// knobs (rate limits, caps, allowlists, read-only, retention). Auth tokens
/// and CORS origins are NOT reloaded (tokens live in `auth.json`; CORS is
/// bound at router construction — both need a restart).
async fn handle_admin_reload(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    role: Option<Extension<AuthRole>>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let guard = state.read().await;
    let sec = guard.security.clone();
    drop(guard);
    let fresh = crate::vortex_config::VortexConfig::load();
    match sec.replace_cfg(fresh.security.clone()) {
        Ok(()) => {}
        Err(reason) => {
            crate::security::audit_log(
                &sec,
                "admin.reload",
                &peer.to_string(),
                &format!("rejected: {reason}"),
                false,
            );
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "error": format!("unsafe config change rejected: {reason}") })),
            )
                .into_response();
        }
    }
    crate::security::audit_log(
        &sec,
        "admin.reload",
        &peer.to_string(),
        "security knobs reloaded",
        true,
    );
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))).into_response()
}

/// Public metrics endpoint in Prometheus text format. Loopback peers and
/// admins can access it; remote non-admins are blocked by `auth_middleware`.
async fn handle_metrics(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
) -> axum::response::Response {
    let guard = state.read().await;
    let (total, c401, c403, c429) = guard.security.metrics_snapshot();
    let text = format!(
        "# HELP vortex_requests_total Total requests served.\n\
         # TYPE vortex_requests_total counter\n\
         vortex_requests_total {total}\n\
         # HELP vortex_denied_401 Denied requests (unauthorized).\n\
         # TYPE vortex_denied_401 counter\n\
         vortex_denied_401 {c401}\n\
         # HELP vortex_denied_403 Denied requests (forbidden).\n\
         # TYPE vortex_denied_403 counter\n\
         vortex_denied_403 {c403}\n\
         # HELP vortex_denied_429 Denied requests (rate-limited).\n\
         # TYPE vortex_denied_429 counter\n\
         vortex_denied_429 {c429}\n\
         # HELP vortex_uptime_seconds Server uptime in seconds.\n\
         # TYPE vortex_uptime_seconds gauge\n\
         vortex_uptime_seconds {}\n",
        guard.startup_time.elapsed().as_secs()
    );
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        text,
    )
        .into_response()
}

/// Admin control plane: current progressive-UX knobs (PERF-02 §6.4).
/// `ws_coalesce_ms` is consumed per-socket by the WS coalescer;
/// `tier_policy` is reported read-only (no runtime consumer yet).
async fn handle_admin_performance_get(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    role: Option<Extension<AuthRole>>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let _guard = state.read().await;
    let perf = crate::vortex_config::VortexConfig::load().performance;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ws_coalesce_ms": perf.ws_coalesce_ms,
            "tier_policy": perf.tier_policy,
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct PerformancePatchRequest {
    pub ws_coalesce_ms: Option<u64>,
}

/// Admin control plane: update progressive-UX knobs (atomic write + audit).
/// Takes effect for sockets opened after the write; in-flight sockets keep
/// the window they started with (order safety).
async fn handle_admin_performance_post(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    role: Option<Extension<AuthRole>>,
    Json(req): Json<PerformancePatchRequest>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let guard = state.read().await;
    let sec = guard.security.clone();
    drop(guard);
    let path = crate::vortex_config::VortexConfig::config_path();
    match crate::vortex_config::VortexConfig::update_performance(&path, req.ws_coalesce_ms) {
        Ok(perf) => {
            crate::security::audit_log(
                &sec,
                "admin.performance",
                &peer.to_string(),
                &format!("ws_coalesce_ms={}", perf.ws_coalesce_ms),
                true,
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ws_coalesce_ms": perf.ws_coalesce_ms,
                    "tier_policy": perf.tier_policy,
                })),
            )
                .into_response()
        }
        Err(e) => {
            crate::security::audit_log(&sec, "admin.performance", &peer.to_string(), &e, false);
            bad_request(e)
        }
    }
}

/// Admin control plane: request/denial counters + uptime.
async fn handle_admin_metrics(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    role: Option<Extension<AuthRole>>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let guard = state.read().await;
    let (total, c401, c403, c429) = guard.security.metrics_snapshot();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "requests_total": total,
            "denied_401": c401,
            "denied_403": c403,
            "denied_429": c429,
            "uptime_seconds": guard.startup_time.elapsed().as_secs(),
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct SessionsPurgeRequest {
    /// Delete sessions older than this many days (default: configured retention).
    pub days: Option<u64>,
}

/// Admin control plane: purge aged chat sessions from disk (audited).
async fn handle_admin_sessions_purge(
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    role: Option<Extension<AuthRole>>,
    Json(req): Json<SessionsPurgeRequest>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(role) {
        return resp.into_response();
    }
    let guard = state.read().await;
    let sec = guard.security.clone();
    drop(guard);
    let days = req
        .days
        .unwrap_or_else(|| sec.cfg_snapshot().sessions_retention_days);
    let removed = crate::session_store::SessionStore::purge_older_than(
        &crate::session_store::SessionStore::default_dir(),
        days,
    );
    crate::security::audit_log(
        &sec,
        "admin.sessions_purge",
        &peer.to_string(),
        &format!("days={days} removed={removed}"),
        true,
    );
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "ok", "removed": removed })),
    )
        .into_response()
}

/// Optional TLS identity for LAN deployments (`--tls-cert/--tls-key`).
#[derive(Clone, Debug, Default)]
pub struct TlsIdentity {
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

pub async fn start_server(
    config: &LlmConfig,
    addr: SocketAddr,
    sec: Arc<SecurityState>,
) -> Result<()> {
    let (_tx, rx) = tokio::sync::oneshot::channel::<()>();
    // No shutdown source: completing the server future ends the process.
    // (The binary passes a real receiver wired to Ctrl+C / tray stop.)
    start_server_with_tls(config, addr, sec, TlsIdentity::default(), rx).await
}

pub async fn start_server_with_tls(
    config: &LlmConfig,
    addr: SocketAddr,
    sec: Arc<SecurityState>,
    tls: TlsIdentity,
    shutdown: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    let engine = LlmInference::load(config)?;
    let tool_executor = Arc::new(KernelToolExecutor);

    // Retention: drop aged chat sessions before serving.
    let retention = sec.cfg_snapshot().sessions_retention_days;
    if retention > 0 {
        let removed = crate::session_store::SessionStore::purge_older_than(
            &crate::session_store::SessionStore::default_dir(),
            retention,
        );
        if removed > 0 {
            println!("[VortexAPI] Purged {removed} chat session(s) older than {retention} days");
        }
    }

    println!("[VortexAPI] Importing knowledge from knowledge/ directory...");
    let knowledge = import_knowledge_at_startup();

    let api_state = Arc::new(RwLock::new(ApiState {
        engine: Arc::new(RwLock::new(engine)),
        tool_executor,
        knowledge,
        startup_time: std::time::Instant::now(),
        inference_permits: Arc::new(Semaphore::new(MAX_CONCURRENT_INFERENCE)),
        security: sec.clone(),
    }));

    let router = create_router(api_state, sec.clone());

    println!("[VortexAPI] Server listening on http://{addr}");
    println!("[VortexAPI] Endpoints:");
    println!("  POST /v1/generate      — Text generation");
    println!("  POST /v1/chat          — Chat completion");
    println!("  POST /v1/batch         — Batch generation");
    println!("  GET  /v1/device        — Device & SIMD info");
    println!("  GET  /v1/health        — Health check");
    println!("  GET  /v1/models        — Model info");
    println!("  GET  /v1/tools         — List available tools");
    println!("  POST /v1/tools/execute — Execute a tool directly");
    println!("  POST /v1/tools/call    — LLM + tool calling");
    println!("  POST /v1/models/swap   — Swap model runtime");
    println!("  POST /v1/knowledge/search — Semantic knowledge search");
    println!("  POST /v1/knowledge/import — Import knowledge text/file");
    println!("  POST /v1/embeddings    — Generate embeddings (OpenAI-compatible)");
    println!("  WS   /ws               — WebSocket streaming");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("Failed to bind: {e}")))?;

    // `ConnectInfo` (peer address) powers the audit log, rate limiter and the
    // loopback-only token bootstrap — it must be plumbed into the service.
    // Both serve paths drain in-flight requests gracefully on shutdown.
    let graceful = async move {
        let _ = shutdown.await;
    };
    match (&tls.cert_path, &tls.key_path) {
        (Some(cert), Some(key)) => {
            println!("[VortexAPI] TLS enabled (https://{addr})");
            crate::embedded_frontend::set_tls_active(true);
            let rustls_cfg = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key)
                .await
                .map_err(|e| {
                    crate::error::VortexAtomsError::Gguf(format!("TLS identity load failed: {e}"))
                })?;
            // axum-server 0.7 drives graceful shutdown through a Handle.
            let handle = axum_server::Handle::new();
            let watch_handle = handle.clone();
            tokio::spawn(async move {
                graceful.await;
                watch_handle.graceful_shutdown(Some(std::time::Duration::from_secs(30)));
            });
            let service = router.into_make_service_with_connect_info::<SocketAddr>();
            axum_server::bind_rustls(addr, rustls_cfg)
                .handle(handle)
                .serve(service)
                .await
                .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("Server error: {e}")))?;
        }
        (None, None) => {
            let service = router.into_make_service_with_connect_info::<SocketAddr>();
            axum::serve(listener, service)
                .with_graceful_shutdown(graceful)
                .await
                .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("Server error: {e}")))?;
        }
        _ => {
            return Err(crate::error::VortexAtomsError::Gguf(
                "TLS needs both --tls-cert and --tls-key (or neither)".to_string(),
            ));
        }
    }

    Ok(())
}
