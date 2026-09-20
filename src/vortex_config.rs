use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::llm_config::LlmConfig;
use crate::security::{AuthConfig, SecurityConfig};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VortexConfig {
    pub server: ServerConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub performance: PerformanceConfig,
}

/// Thread-topology configuration.
///
/// `None` means automatic: inference threads follow available CPUs and async
/// workers default to two on weak hardware.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default)]
    pub infer_threads: Option<usize>,
    #[serde(default)]
    pub async_workers: Option<usize>,
    /// When true, a background thread touches mapped model pages at startup
    /// (1 MiB stride, XOR-accumulate) to prefault them before first inference.
    /// Does not block /v1/health.
    #[serde(default)]
    pub prefault: bool,
    /// Layer double-buffer prefetch: while computing layer N, prefetch layer N+1
    /// via PrefetchVirtualMemory (Windows) / madvise(WILLNEED) (Unix).
    /// Default ON (safe, no correctness impact, 1-3% tok/s on cold pages).
    #[serde(default = "default_layer_prefetch")]
    pub layer_prefetch: bool,
    /// Inference backend: "candle" (default, always available) or "ggml"
    /// (requires `ggml` feature + llama.cpp native build).
    #[serde(default = "default_inference_backend")]
    pub inference_backend: String,
    /// Sliding window KV cache: retain only the last N tokens.
    /// None (default) = full context (no sliding). Useful for long sessions.
    #[serde(default)]
    pub kv_cache_window: Option<usize>,
    /// Static prefix KV cache reuse: cache the KV for a fixed prefix
    /// (e.g., system prompt) and reuse across generations without recomputation.
    /// Requires the prefix to be identical across requests.
    #[serde(default)]
    pub kv_prefix_cache: bool,
    /// KV cache dtype: "f16" (default, memory-efficient) or "f32" (precision).
    #[serde(default = "default_kv_cache_dtype")]
    pub kv_cache_dtype: String,
    /// Speculative decoding: enable n-gram drafter.
    /// Greedy-exact only: the verifier accepts drafts by argmax, so the
    /// runtime automatically skips speculation under temperature/top-k/top-p
    /// sampling (output distribution is then bit-identical to no speculation).
    #[serde(default)]
    pub speculative_enabled: bool,
    /// Maximum tokens to draft per speculative step.
    #[serde(default = "default_max_draft_tokens")]
    pub max_draft_tokens: usize,
    /// Minimum acceptance rate to continue speculative decoding.
    #[serde(default = "default_min_acceptance_rate")]
    pub min_acceptance_rate: f32,
    /// N-gram order for drafter.
    #[serde(default = "default_ngram_order")]
    pub ngram_order: usize,
    /// WebSocket token coalescing window (milliseconds).
    ///
    /// PERF-02 §6.1: token events are batched into fixed windows of at most
    /// `ws_coalesce_ms` before being flushed to the socket (order preserved,
    /// terminal events flush immediately). Default 0 = no coalescing (one
    /// frame per token, the PERF-01 behaviour). Default 50 cuts WS
    /// frame count massively on slow serial engines.
    #[serde(default = "default_ws_coalesce_ms")]
    pub ws_coalesce_ms: u64,
    /// Tier policy for the model ladder ("auto", "eco", "std", "full").
    ///
    /// PERF-02 §6.3: "auto" runs the hardware-aware heuristic, "eco" pins the
    /// cheapest rung (fastpath → 135M), "std" fixes the default rung, "full"
    /// forces the largest affordable. Honor requires
    /// `security.allow_model_routing`.
    #[serde(default = "default_tier_policy")]
    pub tier_policy: String,
}

fn default_kv_cache_dtype() -> String {
    "f16".to_string()
}

fn default_max_draft_tokens() -> usize {
    4
}

fn default_min_acceptance_rate() -> f32 {
    0.5
}

fn default_ngram_order() -> usize {
    3
}

fn default_ws_coalesce_ms() -> u64 {
    50
}

fn default_tier_policy() -> String {
    "auto".to_string()
}

fn default_layer_prefetch() -> bool {
    true
}

fn default_inference_backend() -> String {
    "candle".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 3000,
        }
    }
}

impl VortexConfig {
    /// Load from `vortex.json` in current directory, or return default.
    pub fn load() -> Self {
        let path = PathBuf::from("vortex.json");
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("[VortexConfig] Failed to parse vortex.json: {e}, using defaults");
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("[VortexConfig] Failed to read vortex.json: {e}, using defaults");
                Self::default()
            }
        }
    }

    /// Path of the on-disk config (single source of truth for load + update).
    pub fn config_path() -> PathBuf {
        PathBuf::from("vortex.json")
    }

    /// Upper bound accepted for `ws_coalesce_ms` by the admin endpoint.
    /// 0 = classic per-token frames; above this, frame latency would exceed
    /// any plausible streaming benefit.
    pub const MAX_WS_COALESCE_MS: u64 = 5000;

    /// Validate a requested coalescing window.
    pub fn validate_ws_coalesce_ms(v: u64) -> Result<(), String> {
        if v > Self::MAX_WS_COALESCE_MS {
            return Err(format!(
                "ws_coalesce_ms {v} exceeds maximum {}",
                Self::MAX_WS_COALESCE_MS
            ));
        }
        Ok(())
    }

    /// Read-modify-write of the `performance` table only. Unknown tables and
    /// unknown keys pass through untouched (forward-compatible). Atomic: temp
    /// file + rename, so per-socket `load()` readers never see a half-written
    /// file. `None` fields are left untouched. Returns the resulting
    /// performance table on success.
    pub fn update_performance(
        path: &std::path::Path,
        ws_coalesce_ms: Option<u64>,
    ) -> Result<PerformanceConfig, String> {
        if let Some(v) = ws_coalesce_ms {
            Self::validate_ws_coalesce_ms(v)?;
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let mut doc: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("cannot parse {}: {e}", path.display()))?;
        let table = doc
            .get_mut("performance")
            .and_then(|p| p.as_object_mut())
            .ok_or_else(|| "missing \"performance\" table".to_string())?;
        if let Some(v) = ws_coalesce_ms {
            table.insert("ws_coalesce_ms".to_string(), serde_json::Value::from(v));
        }
        let perf: PerformanceConfig =
            serde_json::from_value(serde_json::Value::Object(table.clone()))
                .map_err(|e| format!("resulting performance table invalid: {e}"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&doc).unwrap_or_default())
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| format!("cannot replace {}: {e}", path.display()))?;
        Ok(perf)
    }
    pub fn addr_string(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
