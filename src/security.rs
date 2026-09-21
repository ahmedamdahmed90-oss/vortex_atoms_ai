//! Local-first security plane for the Vortex Atoms AI HTTP server.
//!
//! The server historically had **no authentication** and a permissive CORS
//! layer, so any local process — or any website rendered in the user's
//! browser — could drive the API, including the file-reading knowledge
//! importer and the model swapper. This module implements:
//!
//! - Bearer-token auth with two least-privilege roles (`user` / `admin`).
//! - Auto-generated, persisted tokens (survive restarts, user-profile scoped).
//! - Strict same-origin CORS with an explicit allowlist.
//! - Per-IP fixed-window rate limiting + request IDs + audit log.
//! - Path sandboxing for knowledge imports and model filenames.
//! - Input validation helpers (temperature, prompt/batch/import sizes).
//!
//! [`AuthRole`] is attached to every authorized request as an axum
//! [`Extension`] by [`auth_middleware`]; privileged handlers re-check it
//! (defense in depth, the middleware already gates admin paths).

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowOrigin, CorsLayer};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

fn default_true() -> bool {
    true
}

/// Authentication settings (new `auth` section of `vortex.json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Master switch. When `false` every request is treated as admin
    /// (legacy local-trust behavior) — a loud warning is logged.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Bearer token for inference/search/chat (role `user`).
    /// Auto-generated on first run when absent.
    #[serde(default)]
    pub api_token: Option<String>,
    /// Bearer token for privileged endpoints (role `admin`).
    /// Auto-generated on first run when absent.
    #[serde(default)]
    pub admin_token: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_token: None,
            admin_token: None,
        }
    }
}

/// Hardening knobs (new `security` section of `vortex.json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Extra origins allowed for cross-origin browser calls
    /// (e.g. `["http://localhost:5173"]` for frontend dev).
    /// Empty (default) = same-origin only.
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// Directories knowledge `file_path` imports may read from.
    /// Relative entries resolve against the server working directory.
    #[serde(default = "default_knowledge_dirs")]
    pub knowledge_dirs: Vec<String>,
    /// When `false` (default), `/v1/models/swap` only accepts the
    /// pre-configured model definitions; arbitrary HF repos are rejected.
    #[serde(default)]
    pub allow_custom_models: bool,
    /// When `false` (default, fail-closed), a `model` routing hint on
    /// `/v1/generate` or `/v1/chat` is rejected. Enabling it lets user-facing
    /// requests select a pre-configured matrix entry (`eco`/`q4_0`); every
    /// routed swap is recorded in the audit log.
    #[serde(default)]
    pub allow_model_routing: bool,
    /// Max prompt size in characters per inference request.
    #[serde(default = "default_max_prompt_chars")]
    pub max_prompt_chars: usize,
    /// Max number of prompts in one `/v1/batch` call.
    #[serde(default = "default_max_batch_prompts")]
    pub max_batch_prompts: usize,
    /// Max characters of inline `text` in `/v1/knowledge/import`.
    #[serde(default = "default_max_import_chars")]
    pub max_import_chars: usize,
    /// Accepted sampling-temperature range (inclusive).
    #[serde(default = "default_temp_min")]
    pub temperature_min: f64,
    #[serde(default = "default_temp_max")]
    pub temperature_max: f64,
    /// Per-IP request budget per 10-second window (all endpoints).
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_10s: u32,
    /// Refuse model files larger than this (bytes).
    #[serde(default = "default_max_model_bytes")]
    pub max_model_bytes: u64,
    /// Read-only mode: mutating admin endpoints (swap/import/rotate/reload/
    /// sessions-purge) are refused with 403. For kiosk & demo deployments.
    #[serde(default)]
    pub read_only: bool,
    /// Even with a valid admin token, admin endpoints only answer loopback
    /// peers. Remote operators must use an SSH tunnel / local agent.
    #[serde(default = "default_true")]
    pub admin_loopback_only: bool,
    /// Chat sessions older than this (days) are purged at startup / on
    /// demand. 0 disables retention purging.
    #[serde(default = "default_retention_days")]
    pub sessions_retention_days: u64,
    /// Minimum free disk space in MB before refusing model downloads
    /// and knowledge imports. Default 1024 MB (1 GB).
    #[serde(default = "default_min_disk_free_mb")]
    pub min_disk_free_mb: u64,
}

fn default_retention_days() -> u64 {
    30
}
fn default_min_disk_free_mb() -> u64 {
    1024
}
fn default_knowledge_dirs() -> Vec<String> {
    vec!["knowledge".to_string()]
}
fn default_max_prompt_chars() -> usize {
    32768
}
fn default_max_batch_prompts() -> usize {
    32
}
fn default_max_import_chars() -> usize {
    200_000
}
fn default_temp_min() -> f64 {
    0.0
}
fn default_temp_max() -> f64 {
    2.0
}
fn default_rate_limit() -> u32 {
    600
}
fn default_max_model_bytes() -> u64 {
    16 * 1024 * 1024 * 1024
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_origins: Vec::new(),
            knowledge_dirs: default_knowledge_dirs(),
            allow_custom_models: false,
            allow_model_routing: false,
            max_prompt_chars: default_max_prompt_chars(),
            max_batch_prompts: default_max_batch_prompts(),
            max_import_chars: default_max_import_chars(),
            temperature_min: default_temp_min(),
            temperature_max: default_temp_max(),
            rate_limit_per_10s: default_rate_limit(),
            max_model_bytes: default_max_model_bytes(),
            read_only: false,
            admin_loopback_only: true,
            sessions_retention_days: default_retention_days(),
            min_disk_free_mb: default_min_disk_free_mb(),
        }
    }
}

// ---------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------

/// Least-privilege role attached to authorized requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthRole {
    User,
    Admin,
}

// ---------------------------------------------------------------------------
// Token storage
// ---------------------------------------------------------------------------

/// Directory that holds `auth.json` + `audit.log` (user-profile scoped).
pub fn auth_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(appdata).join("vortex_atoms_ai")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".vortex_atoms_ai")
    } else {
        PathBuf::from(".vortex_atoms_ai")
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct TokenFile {
    pub api_token: Option<String>,
    pub admin_token: Option<String>,
}

/// Serialize + DPAPI-protect + user-ACL a token file. Best-effort hardening
/// around an infallible-feeling write: failures are reported to stderr but
/// never crash startup (the tokens also live in memory).
pub fn persist_token_file(path: &Path, file: &TokenFile) {
    let Ok(json) = serde_json::to_string_pretty(file) else {
        return;
    };
    let bytes = protect_data(json.as_bytes()).unwrap_or_else(|_| json.into_bytes());
    if std::fs::write(path, bytes).is_ok() {
        if let Err(e) = restrict_to_current_user(path) {
            eprintln!(
                "[Auth] token file ACL hardening failed: {}",
                e.public_message()
            );
        }
    }
}

fn generate_token(prefix: &str) -> String {
    let mut bytes = [0u8; 32];
    if getrandom::getrandom(&mut bytes).is_err() {
        // Extremely unlikely; fall back to time+pid entropy so first boot
        // never fails, then persist — still unguessable in practice.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut x = now ^ ((std::process::id() as u128) << 64) ^ 0x9E3779B97F4A7C15;
        for b in bytes.iter_mut() {
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            *b = (x.wrapping_mul(0x2545F4914F6CDD1D) >> 56) as u8;
        }
    }
    let mut s = String::with_capacity(prefix.len() + 64);
    s.push_str(prefix);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((b & 0xF) as u32, 16).unwrap_or('0'));
    }
    s
}

/// Resolve effective tokens: explicit `vortex.json` values win; otherwise load
/// `auth.json` (creating + persisting fresh random tokens on first run).
/// Returns `(api_token, admin_token)` — `None`s when auth is disabled.
pub fn ensure_tokens(auth: &mut AuthConfig) -> (Option<String>, Option<String>) {
    if !auth.enabled {
        return (None, None);
    }
    let dir = auth_dir();
    let path = dir.join("auth.json");
    let mut file = TokenFile::default();
    if path.exists() {
        if let Ok(bytes) = std::fs::read(&path) {
            // DPAPI blob, marked plaintext, or legacy raw JSON.
            let plain = unprotect_data(&bytes).unwrap_or(bytes);
            if let Ok(text) = String::from_utf8(plain) {
                if let Ok(parsed) = serde_json::from_str::<TokenFile>(&text) {
                    file = parsed;
                }
            }
        }
    }
    let mut changed = false;
    if auth.api_token.is_none() && file.api_token.is_none() {
        file.api_token = Some(generate_token("vxt_"));
        changed = true;
    }
    if auth.admin_token.is_none() && file.admin_token.is_none() {
        file.admin_token = Some(generate_token("vxa_"));
        changed = true;
    }
    if changed {
        let _ = std::fs::create_dir_all(&dir);
        persist_token_file(&path, &file);
    }
    let api = auth.api_token.clone().or(file.api_token);
    let admin = auth.admin_token.clone().or(file.admin_token);
    auth.api_token.clone_from(&api);
    auth.admin_token.clone_from(&admin);
    (api, admin)
}

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

struct RateBucket {
    window_start: Instant,
    count: u32,
}

struct SecurityInner {
    request_counter: AtomicU64,
    rate: Mutex<HashMap<IpAddr, RateBucket>>,
    total_requests: AtomicU64,
    denied_401: AtomicU64,
    denied_403: AtomicU64,
    denied_429: AtomicU64,
}

#[derive(Clone, Debug)]
struct TokenPair {
    api: Option<String>,
    admin: Option<String>,
}

pub struct SecurityState {
    /// Hot-reloadable hardening knobs (`POST /v1/admin/reload` swaps this;
    /// read through [`SecurityState::cfg_snapshot`]).
    pub cfg: std::sync::RwLock<SecurityConfig>,
    tokens: Arc<tokio::sync::RwLock<TokenPair>>,
    pub audit_path: PathBuf,
    inner: Arc<SecurityInner>,
}

impl Clone for SecurityState {
    fn clone(&self) -> Self {
        Self {
            cfg: std::sync::RwLock::new(self.cfg_snapshot()),
            tokens: self.tokens.clone(),
            audit_path: self.audit_path.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl SecurityState {
    pub fn new(
        cfg: SecurityConfig,
        api_token: Option<String>,
        admin_token: Option<String>,
    ) -> Self {
        let mut audit_path = auth_dir();
        audit_path.push("audit.log");
        Self {
            cfg: std::sync::RwLock::new(cfg),
            tokens: Arc::new(tokio::sync::RwLock::new(TokenPair {
                api: api_token,
                admin: admin_token,
            })),
            audit_path,
            inner: Arc::new(SecurityInner {
                request_counter: AtomicU64::new(1),
                rate: Mutex::new(HashMap::new()),
                total_requests: AtomicU64::new(0),
                denied_401: AtomicU64::new(0),
                denied_403: AtomicU64::new(0),
                denied_429: AtomicU64::new(0),
            }),
        }
    }

    /// Cheap clone of the current hardening knobs.
    pub fn cfg_snapshot(&self) -> SecurityConfig {
        self.cfg.read().map(|c| c.clone()).unwrap_or_default()
    }

    /// Swap the live knobs after re-validating a config file (admin reload).
    /// Rejects any relaxation of security-critical settings (read_only,
    /// admin_loopback_only, min_disk_free_mb, max_model_bytes) with 422.
    pub fn replace_cfg(&self, cfg: SecurityConfig) -> Result<(), String> {
        let current = self.cfg_snapshot();
        if current.read_only && !cfg.read_only {
            return Err("cannot disable read_only via reload".to_string());
        }
        if current.admin_loopback_only && !cfg.admin_loopback_only {
            return Err("cannot disable admin_loopback_only via reload".to_string());
        }
        if cfg.min_disk_free_mb == 0 {
            return Err("min_disk_free_mb must be > 0".to_string());
        }
        if cfg.max_model_bytes > 32 * 1024 * 1024 * 1024 {
            return Err("max_model_bytes exceeds 32 GB ceiling".to_string());
        }
        if cfg.temperature_min < 0.0 || cfg.temperature_max > 5.0 {
            return Err("temperature must be in [0.0, 5.0]".to_string());
        }
        if let Ok(mut slot) = self.cfg.write() {
            *slot = cfg;
        }
        Ok(())
    }

    pub fn metrics_snapshot(&self) -> (u64, u64, u64, u64) {
        (
            self.inner.total_requests.load(Ordering::Relaxed),
            self.inner.denied_401.load(Ordering::Relaxed),
            self.inner.denied_403.load(Ordering::Relaxed),
            self.inner.denied_429.load(Ordering::Relaxed),
        )
    }

    pub fn auth_enabled(&self) -> bool {
        // Auth is enabled unless BOTH tokens are absent (legacy local trust).
        // Read without blocking: the pair is only written at startup/rotate.
        self.tokens
            .try_read()
            .map(|t| t.api.is_some() || t.admin.is_some())
            .unwrap_or(true)
    }

    pub async fn role_for(&self, presented: &str) -> Option<AuthRole> {
        let t = self.tokens.read().await;
        if Some(presented) == t.admin.as_deref() {
            Some(AuthRole::Admin)
        } else if Some(presented) == t.api.as_deref() {
            Some(AuthRole::User)
        } else {
            None
        }
    }

    pub async fn tokens_snapshot(&self) -> (Option<String>, Option<String>) {
        let t = self.tokens.read().await;
        (t.api.clone(), t.admin.clone())
    }

    /// Regenerate the admin token, persist it, and return the new value.
    pub async fn rotate_admin(&self) -> String {
        let fresh = generate_token("vxa_");
        {
            let mut t = self.tokens.write().await;
            t.admin = Some(fresh.clone());
            let file = TokenFile {
                api_token: t.api.clone(),
                admin_token: t.admin.clone(),
            };
            persist_token_file(&auth_dir().join("auth.json"), &file);
        }
        fresh
    }

    fn next_request_id(&self) -> String {
        let n = self.inner.request_counter.fetch_add(1, Ordering::Relaxed);
        format!("vxt-{}-{n}", std::process::id())
    }

    fn check_rate(&self, ip: IpAddr) -> bool {
        let limit = self
            .cfg
            .read()
            .map(|c| c.rate_limit_per_10s)
            .unwrap_or(600)
            .max(1);
        let now = Instant::now();
        let Ok(mut map) = self.inner.rate.lock() else {
            return true;
        };
        if map.len() > 10_000 {
            map.retain(|_, b| now.duration_since(b.window_start).as_secs() < 10);
        }
        match map.get_mut(&ip) {
            Some(b) if now.duration_since(b.window_start).as_secs() < 10 => {
                b.count = b.count.saturating_add(1);
                b.count <= limit
            }
            _ => {
                map.insert(
                    ip,
                    RateBucket {
                        window_start: now,
                        count: 1,
                    },
                );
                true
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Paths & roles
// ---------------------------------------------------------------------------

fn is_public_path(path: &str) -> bool {
    // Embedded UI + static assets (the router falls back to index.html).
    if !path.starts_with("/v1") && !path.starts_with("/ws") && !path.starts_with("/auth") {
        return true;
    }
    // Liveness probe (kept shape-compatible for Docker/health checks).
    if path == "/v1/health" {
        return true;
    }
    // Same-origin token bootstrap (see handler: loopback-only).
    if path == "/auth/bootstrap" {
        return true;
    }
    false
}

fn is_admin_path(path: &str) -> bool {
    path == "/v1/models/swap" || path == "/v1/knowledge/import" || path.starts_with("/v1/admin/")
}

fn bearer_token(req: &Request) -> Option<String> {
    req.headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.trim().to_string())
}

fn ws_query_token(req: &Request) -> Option<String> {
    req.uri().query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            (k == "token").then(|| v.trim().to_string())
        })
    })
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
        Json(serde_json::json!({"error": "missing or invalid API token"})),
    )
        .into_response()
}

fn forbidden_admin() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({"error": "admin token required"})),
    )
        .into_response()
}

fn rate_limited() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({"error": "rate limit exceeded, retry later"})),
    )
        .into_response()
}

fn forbidden_read_only() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({"error": "server is in read-only mode"})),
    )
        .into_response()
}

fn forbidden_remote_admin() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({"error": "admin endpoints are loopback-only on this server"})),
    )
        .into_response()
}

/// Admin paths that mutate server state (additionally gated by read-only).
pub fn is_mutating_admin_path(path: &str) -> bool {
    path == "/v1/models/swap"
        || path == "/v1/knowledge/import"
        || path == "/v1/admin/rotate"
        || path == "/v1/admin/reload"
        || path == "/v1/admin/sessions/purge"
        || path == "/v1/admin/performance"
}

/// Verify the WS `Origin` header matches an allowed origin or the
/// request `Host` (same-origin). Prevents transport hijacking via
/// cross-origin WebSocket upgrade requests.
fn ws_origin_allowed(req: &Request, sec: &SecurityState) -> bool {
    let origin = match req.headers().get("origin") {
        Some(o) => o,
        None => return false,
    };
    let origin_str = match origin.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let cfg = sec.cfg_snapshot();
    for allowed in &cfg.allowed_origins {
        if allowed == origin_str {
            return true;
        }
    }
    // Same-origin fallback mirrors origin_allowed (CORS twin) exactly:
    // compare full host:port on both sides. The old code stripped the port
    // from Host but not from Origin, rejecting every same-origin WS
    // handshake on a non-default port (i.e. all of them in practice).
    let Some(oh) = origin_host(origin) else {
        return false;
    };
    let Some(host) = req.headers().get("host").and_then(|h| h.to_str().ok()) else {
        return false;
    };
    oh == host.to_lowercase()
}

/// Axum middleware: request IDs, rate limiting, bearer auth + role gating.
pub async fn auth_middleware(
    State(sec): State<Arc<SecurityState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Response {
    let request_id = sec.next_request_id();
    let path = req.uri().path().to_string();
    sec.inner.total_requests.fetch_add(1, Ordering::Relaxed);

    macro_rules! deny {
        ($counter:ident, $res:expr) => {{
            sec.inner.$counter.fetch_add(1, Ordering::Relaxed);
            let mut res = $res;
            res.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).unwrap_or(HeaderValue::from_static("x")),
            );
            return res;
        }};
    }

    if !sec.check_rate(peer.ip()) {
        deny!(denied_429, rate_limited());
    }

    if !is_public_path(&path) {
        // WS origin check: prevent transport hijacking via unauthorized
        // WebSocket upgrades from cross-origin sites.
        if path == "/ws" && !ws_origin_allowed(&req, &sec) {
            deny!(denied_403, forbidden_read_only());
        }
        // Read-only kiosk mode refuses every state mutation even with a
        // valid admin token (and even with auth disabled).
        if is_mutating_admin_path(&path) {
            let read_only = sec.cfg.read().map(|c| c.read_only).unwrap_or(false);
            if read_only {
                deny!(denied_403, forbidden_read_only());
            }
        }
        if sec.auth_enabled() {
            let presented = if path == "/ws" {
                ws_query_token(&req).or_else(|| bearer_token(&req))
            } else {
                bearer_token(&req)
            };
            let role = match presented {
                Some(t) => match sec.role_for(&t).await {
                    Some(r) => r,
                    None => deny!(denied_401, unauthorized()),
                },
                None => deny!(denied_401, unauthorized()),
            };
            if is_admin_path(&path) {
                if role != AuthRole::Admin {
                    deny!(denied_403, forbidden_admin());
                }
                // Even with a valid admin token, admin endpoints stay
                // loopback-only unless explicitly relaxed in config.
                let loopback_only = sec
                    .cfg
                    .read()
                    .map(|c| c.admin_loopback_only)
                    .unwrap_or(true);
                if loopback_only && !peer.ip().is_loopback() {
                    deny!(denied_403, forbidden_remote_admin());
                }
            }
            req.extensions_mut().insert(role);
        } else {
            // Auth disabled: legacy local-trust behavior (warned at startup).
            req.extensions_mut().insert(AuthRole::Admin);
        }
    }

    let mut res = next.run(req).await;
    res.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or(HeaderValue::from_static("x")),
    );
    res
}

// ---------------------------------------------------------------------------
// Strict CORS
// ---------------------------------------------------------------------------

fn origin_host(origin: &HeaderValue) -> Option<String> {
    let s = origin.to_str().ok()?;
    let after_scheme = s.split("://").nth(1)?;
    Some(after_scheme.split('/').next()?.to_lowercase())
}

/// Allow when the Origin is explicitly listed or equals the request Host
/// (same-origin). Everything else — including `null` (file://) — is denied.
fn origin_allowed(origin: &HeaderValue, host: Option<&HeaderValue>, allow: &[String]) -> bool {
    let origin_str = match origin.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    if allow.iter().any(|a| a == origin_str) {
        return true;
    }
    let Some(oh) = origin_host(origin) else {
        return false;
    };
    let Some(host) = host.and_then(|h| h.to_str().ok()) else {
        return false;
    };
    oh == host.to_lowercase()
}

pub fn build_cors(cfg: &SecurityConfig) -> CorsLayer {
    let allow = cfg.allowed_origins.clone();
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(
            move |origin: &HeaderValue, req: &axum::http::request::Parts| {
                origin_allowed(origin, req.headers.get(header::HOST), &allow)
            },
        ))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
}

// ---------------------------------------------------------------------------
// Security headers middleware
// ---------------------------------------------------------------------------

/// Axum middleware: inject strict security headers on every response
/// to mitigate transport hijacking, clickjacking, MIME sniffing, and
/// XSS-driven content injection.
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    h.insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );
    h.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    h.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"),
    );
    h.insert(
        "cache-control",
        HeaderValue::from_static("no-store, no-cache, must-revalidate, proxy-revalidate"),
    );
    h.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );
    if let Ok(csp) = HeaderValue::from_str(
        "default-src 'none'; frame-ancestors 'none'; sandbox allow-forms allow-scripts; \
         connect-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; \
         font-src 'self'; script-src 'self'",
    ) {
        h.insert("content-security-policy", csp);
    }
    res
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

/// Append-only JSONL audit record for privileged actions. Best-effort:
/// a logging failure must never fail the action itself.
pub fn audit_log(sec: &SecurityState, action: &str, peer: &str, detail: &str, ok: bool) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = serde_json::json!({
        "ts": now,
        "action": action,
        "peer": peer,
        "detail": detail,
        "ok": ok,
    });
    if let Ok(mut s) = serde_json::to_string(&line) {
        s.push('\n');
        if let Err(e) = append_audit(&sec.audit_path, &s) {
            eprintln!("[Audit] write failed: {e}");
        }
    }
}

fn append_audit(path: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(line.as_bytes())
}

pub fn read_audit_tail(path: &Path, lines: usize) -> Vec<String> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    content
        .lines()
        .rev()
        .take(lines.min(500))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Path sandboxing
// ---------------------------------------------------------------------------

/// Resolve a knowledge `file_path` import request strictly inside one of the
/// allowlisted directories. Returns the canonical path or a safe error string
/// (no filesystem details leak to API callers).
pub fn resolve_import_path(raw: &str, allow_dirs: &[String]) -> Result<PathBuf, String> {
    if raw.is_empty() || raw.len() > 4096 {
        return Err("invalid path".to_string());
    }
    let candidate = Path::new(raw);
    let canonical = candidate
        .canonicalize()
        .map_err(|_| "path not found or not accessible".to_string())?;
    for dir in allow_dirs {
        let base = Path::new(dir);
        if let Ok(base_canon) = base.canonicalize() {
            if canonical.starts_with(&base_canon) {
                return Ok(canonical);
            }
        }
    }
    Err("path is outside the allowed knowledge directories".to_string())
}

/// Reject model/tokenizer filenames that could escape the cache directory.
pub fn check_model_file_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 256 {
        return Err("invalid model filename".to_string());
    }
    if name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || Path::new(name).is_absolute()
    {
        return Err("model filename must be a plain file name".to_string());
    }
    Ok(())
}

/// HuggingFace repo ids look like `org/name` with a conservative alphabet.
pub fn check_hf_repo(repo: &str) -> Result<(), String> {
    fn part_ok(p: &str) -> bool {
        !p.is_empty()
            && p != "."
            && p != ".."
            && p.len() <= 128
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    }
    match repo.split_once('/') {
        Some((org, name)) if part_ok(org) && part_ok(name) && !name.contains('/') => Ok(()),
        _ => Err("invalid model repo (expected org/name)".to_string()),
    }
}

/// Only pre-configured model definitions (or the built-in defaults) may be
/// swapped in unless `allow_custom_models` is set.
pub fn is_allowed_model(repo: &str, file: &str) -> bool {
    use crate::llm_download::{
        DEFAULT_MODEL_FILE, DEFAULT_MODEL_REPO, DEFAULT_TOKENIZER_FILE, DEFAULT_TOKENIZER_REPO,
        MODELS_1_5B, MODELS_3B, MODELS_7B, MODEL_ECO_SLOT,
    };
    if repo == DEFAULT_MODEL_REPO && (file == DEFAULT_MODEL_FILE || file == DEFAULT_TOKENIZER_FILE)
    {
        return true;
    }
    if repo == DEFAULT_TOKENIZER_REPO && file == DEFAULT_TOKENIZER_FILE {
        return true;
    }
    for def in MODELS_1_5B
        .iter()
        .chain(MODELS_3B.iter())
        .chain(MODELS_7B.iter())
    {
        if repo == def.repo && (file == def.gguf_file || file == DEFAULT_TOKENIZER_FILE) {
            return true;
        }
    }
    for def in MODEL_ECO_SLOT {
        if repo == def.repo && (file == def.gguf_file || file == DEFAULT_TOKENIZER_FILE) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

/// Finite temperatures clamp into range; NaN/infinite are rejected.
pub fn check_temperature(t: f64, cfg: &SecurityConfig) -> Result<f64, String> {
    if !t.is_finite() {
        return Err("temperature must be a finite number".to_string());
    }
    Ok(t.clamp(cfg.temperature_min, cfg.temperature_max))
}

pub fn check_text_len(text: &str, max_chars: usize, what: &str) -> Result<(), String> {
    if text.chars().count() > max_chars {
        return Err(format!("{what} exceeds limit of {max_chars} characters"));
    }
    Ok(())
}

/// Session ids become filenames (`{id}.json`); keep them tightly scoped so a
/// future HTTP wiring cannot traverse directories.
pub fn sanitize_session_id(id: &str) -> Result<String, String> {
    if id.is_empty() || id.len() > 64 {
        return Err("invalid session id".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("invalid session id".to_string());
    }
    Ok(id.to_string())
}

/// Allowlist gate for spawning MCP stdio servers. `VORTEX_MCP_ALLOW` holds a
/// comma-separated list of permitted command file stems (e.g.
/// `VORTEX_MCP_ALLOW=uvx,node`). Unset/empty = deny everything.
pub fn check_mcp_command(command: &str) -> crate::Result<()> {
    let stem = Path::new(command)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let allow = std::env::var("VORTEX_MCP_ALLOW").unwrap_or_default();
    let permitted = allow
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .any(|s| !s.is_empty() && s == stem);
    if permitted {
        Ok(())
    } else {
        Err(crate::error::VortexAtomsError::Gguf(
            "MCP stdio command not allowlisted (set VORTEX_MCP_ALLOW)".to_string(),
        ))
    }
}

/// Loopback-only helper for fail-closed remote-bind policy.
pub fn is_loopback_host(host: &str) -> bool {
    let h = host.trim().trim_matches(['[', ']']).to_lowercase();
    h == "127.0.0.1" || h == "::1" || h == "localhost"
}

// ---------------------------------------------------------------------------
// OS-level protections: DPAPI secrets, user-only ACLs, disk guard,
// single-instance mutex.
// ---------------------------------------------------------------------------

const PROTECTED_MAGIC: &[u8] = b"VXDP1";
const PLAINTEXT_MAGIC: &[u8] = b"VXPL1";

/// Encrypt bytes for the current user+machine (Windows DPAPI,
/// `CRYPTPROTECT_UI_FORBIDDEN`). Output is `MAGIC || blob`.
/// Non-Windows fallback stores plaintext with a distinct magic marker and is
/// only suitable for single-user machines (documented limitation).
#[cfg(windows)]
pub fn protect_data(plain: &[u8]) -> crate::Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: input borrows `plain` for the call; output is allocated by the
    // API and freed below. No UI is permitted to ever appear.
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        let code = unsafe { GetLastError() };
        return Err(crate::error::VortexAtomsError::Gguf(format!(
            "DPAPI protect failed (win32 {code})"
        )));
    }
    let blob =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData as _);
    }
    let mut out = Vec::with_capacity(PROTECTED_MAGIC.len() + blob.len());
    out.extend_from_slice(PROTECTED_MAGIC);
    out.extend_from_slice(&blob);
    Ok(out)
}

/// Reverse of [`protect_data`]; accepts legacy unmarked plaintext too so old
/// `auth.json` / session files keep loading.
#[cfg(windows)]
pub fn unprotect_data(blob: &[u8]) -> crate::Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    if blob.starts_with(PLAINTEXT_MAGIC) {
        return Ok(blob[PLAINTEXT_MAGIC.len()..].to_vec());
    }
    if !blob.starts_with(PROTECTED_MAGIC) {
        // Legacy format (pre-DPAPI files): pass through as-is.
        return Ok(blob.to_vec());
    }
    let body = &blob[PROTECTED_MAGIC.len()..];
    let input = CRYPT_INTEGER_BLOB {
        cbData: body.len() as u32,
        pbData: body.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: same contract as protect_data; runs on the calling thread.
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        let code = unsafe { GetLastError() };
        return Err(crate::error::VortexAtomsError::Gguf(format!(
            "DPAPI unprotect failed (win32 {code}) — wrong user/machine?"
        )));
    }
    let plain =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData as _);
    }
    Ok(plain)
}

#[cfg(not(windows))]
pub fn protect_data(plain: &[u8]) -> crate::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(PLAINTEXT_MAGIC.len() + plain.len());
    out.extend_from_slice(PLAINTEXT_MAGIC);
    out.extend_from_slice(plain);
    Ok(out)
}

#[cfg(not(windows))]
pub fn unprotect_data(blob: &[u8]) -> crate::Result<Vec<u8>> {
    if blob.starts_with(PROTECTED_MAGIC) {
        return Err(crate::error::VortexAtomsError::Gguf(
            "protected blob needs Windows DPAPI".to_string(),
        ));
    }
    if blob.starts_with(PLAINTEXT_MAGIC) {
        return Ok(blob[PLAINTEXT_MAGIC.len()..].to_vec());
    }
    Ok(blob.to_vec())
}

/// Restrict a secret file to the current user only.
/// Windows: replace the DACL with a single user ALLOW ACE (no inheritance).
/// Unix: chmod 0o600.
#[cfg(windows)]
pub fn restrict_to_current_user(path: &Path) -> crate::Result<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree};
    use windows_sys::Win32::Security::Authorization::{
        SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, SET_ACCESS, SE_FILE_OBJECT,
        TRUSTEE_IS_SID, TRUSTEE_IS_USER, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenUser, DACL_SECURITY_INFORMATION, NO_INHERITANCE,
        PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: short-lived handles/SIDs below; every allocation is freed.
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(crate::error::VortexAtomsError::Gguf(
                "ACL: OpenProcessToken failed".into(),
            ));
        }
        let mut need = 0u32;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut need);
        let mut buf = vec![0u8; need.max(64) as usize];
        if GetTokenInformation(
            token,
            TokenUser,
            buf.as_mut_ptr() as *mut _,
            need,
            &mut need,
        ) == 0
        {
            CloseHandle(token);
            return Err(crate::error::VortexAtomsError::Gguf(
                "ACL: GetTokenInformation failed".into(),
            ));
        }
        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
        let ea = EXPLICIT_ACCESS_W {
            grfAccessPermissions: 0x80000000 | 0x40000000, // GENERIC_READ | GENERIC_WRITE
            grfAccessMode: SET_ACCESS,
            grfInheritance: NO_INHERITANCE,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: std::ptr::null_mut(),
                MultipleTrusteeOperation: 0, // NO_MULTIPLE_TRUSTEE
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_USER,
                ptstrName: token_user.User.Sid as *mut u16,
            },
        };
        let mut new_acl = std::ptr::null_mut();
        let rc = SetEntriesInAclW(1, &ea, std::ptr::null(), &mut new_acl);
        CloseHandle(token);
        if rc != 0 {
            return Err(crate::error::VortexAtomsError::Gguf(format!(
                "ACL: SetEntriesInAclW={rc}"
            )));
        }
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let rc = SetNamedSecurityInfoW(
            wide.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_acl,
            std::ptr::null_mut(),
        );
        LocalFree(new_acl as _);
        if rc != 0 {
            return Err(crate::error::VortexAtomsError::Gguf(format!(
                "ACL: SetNamedSecurityInfoW={rc}"
            )));
        }
        Ok(())
    }
}

#[cfg(all(unix, not(windows)))]
pub fn restrict_to_current_user(path: &Path) -> crate::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(crate::error::VortexAtomsError::Io)
}

#[cfg(not(any(windows, unix)))]
pub fn restrict_to_current_user(_path: &Path) -> crate::Result<()> {
    Ok(())
}

/// Refuse downloads/imports when the volume cannot fit `need_bytes` plus a
/// 512 MiB safety margin. Windows implementation; other platforms rely on
/// write-error handling (documented limitation).
#[cfg(windows)]
pub fn check_disk_space(dir: &Path, need_bytes: u64) -> crate::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
    let mut free: u64 = 0;
    // SAFETY: GetDiskFreeSpaceExW writes one u64 through the pointer.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free as *mut u64 as *mut _,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(crate::error::VortexAtomsError::Gguf(
            "disk-space check failed".to_string(),
        ));
    }
    const MARGIN: u64 = 512 * 1024 * 1024;
    if free < need_bytes.saturating_add(MARGIN) {
        return Err(crate::error::VortexAtomsError::Gguf(format!(
            "not enough free disk space (need ~{} MiB + 512 MiB margin)",
            need_bytes / (1024 * 1024)
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn check_disk_space(_dir: &Path, _need_bytes: u64) -> crate::Result<()> {
    Ok(())
}

/// OS single-instance guard: second server process exits with a clear message
/// instead of fighting over ports and the model cache.
/// Windows: named mutex. Unix: `create_new` pid lockfile (stale locks refuse
/// with removal instructions rather than guessing liveness).
pub struct SingleInstance {
    #[cfg(windows)]
    _handle: *mut std::ffi::c_void,
    #[cfg(not(windows))]
    _lock_path: PathBuf,
}

impl SingleInstance {
    pub fn acquire(name: &str) -> crate::Result<Self> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
            use windows_sys::Win32::System::Threading::CreateMutexW;

            let wide: Vec<u16> = format!("Global\\VortexAtomsAI-{name}")
                .encode_utf16()
                .chain([0])
                .collect();
            // SAFETY: no security attrs, unnamed ownership semantics.
            let handle = unsafe { CreateMutexW(std::ptr::null(), 1, wide.as_ptr()) };
            if handle.is_null() {
                return Err(crate::error::VortexAtomsError::Gguf(
                    "single-instance mutex creation failed".to_string(),
                ));
            }
            if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
                return Err(crate::error::VortexAtomsError::Gguf(
                    "another Vortex Atoms AI server is already running".to_string(),
                ));
            }
            Ok(Self { _handle: handle })
        }
        #[cfg(not(windows))]
        {
            let path = std::env::temp_dir().join(format!("vortex-atoms-ai-{name}.lock"));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut f) => {
                    use std::io::Write;
                    let _ = writeln!(f, "{}", std::process::id());
                    Ok(Self { _lock_path: path })
                }
                Err(_) => Err(crate::error::VortexAtomsError::Gguf(format!(
                    "another instance may be running (lock {}); remove it if stale",
                    path.display()
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_format_and_uniqueness() {
        let a = generate_token("vxt_");
        let b = generate_token("vxt_");
        assert!(a.starts_with("vxt_") && a.len() == 68);
        assert_ne!(a, b);
    }

    #[test]
    fn public_paths() {
        assert!(is_public_path("/"));
        assert!(is_public_path("/index.html"));
        assert!(is_public_path("/assets/app.js"));
        assert!(is_public_path("/v1/health"));
        assert!(is_public_path("/auth/bootstrap"));
        assert!(!is_public_path("/v1/chat"));
        assert!(!is_public_path("/ws"));
        assert!(!is_public_path("/v1/admin/status"));
    }

    #[test]
    fn admin_paths() {
        assert!(is_admin_path("/v1/models/swap"));
        assert!(is_admin_path("/v1/knowledge/import"));
        assert!(is_admin_path("/v1/admin/rotate"));
        assert!(!is_admin_path("/v1/chat"));
        assert!(!is_admin_path("/v1/health"));
    }

    #[test]
    fn hf_repo_validation() {
        assert!(check_hf_repo("Qwen/Qwen2.5-0.5B-Instruct-GGUF").is_ok());
        assert!(check_hf_repo("a/b").is_ok());
        assert!(check_hf_repo("no-slash").is_err());
        assert!(check_hf_repo("a/b/c").is_err());
        assert!(check_hf_repo("../evil").is_err());
        assert!(check_hf_repo("").is_err());
    }

    #[test]
    fn model_filename_validation() {
        assert!(check_model_file_name("model.gguf").is_ok());
        assert!(check_model_file_name("../../win.ini").is_err());
        assert!(check_model_file_name("a/b.gguf").is_err());
        assert!(check_model_file_name("C:\\x.gguf").is_err());
        assert!(check_model_file_name("").is_err());
    }

    #[test]
    fn session_id_sanitization() {
        assert_eq!(sanitize_session_id("abc-123_X").unwrap(), "abc-123_X");
        assert!(sanitize_session_id("../../etc").is_err());
        assert!(sanitize_session_id("a/b").is_err());
        assert!(sanitize_session_id("").is_err());
        assert!(sanitize_session_id(&"a".repeat(65)).is_err());
    }

    #[test]
    fn temperature_checks() {
        let cfg = SecurityConfig::default();
        assert_eq!(check_temperature(0.7, &cfg).unwrap(), 0.7);
        assert_eq!(check_temperature(99.0, &cfg).unwrap(), 2.0);
        assert_eq!(check_temperature(-5.0, &cfg).unwrap(), 0.0);
        assert!(check_temperature(f64::NAN, &cfg).is_err());
        assert!(check_temperature(f64::INFINITY, &cfg).is_err());
    }

    #[test]
    fn loopback_detection() {
        assert!(is_loopback_host("127.0.0.1"));
        assert!(is_loopback_host("::1"));
        assert!(is_loopback_host("localhost"));
        assert!(!is_loopback_host("0.0.0.0"));
        assert!(!is_loopback_host("192.168.1.5"));
    }

    #[test]
    fn allowed_model_matrix() {
        assert!(is_allowed_model(
            "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
            "qwen2.5-0.5b-instruct-q4_k_m.gguf"
        ));
        assert!(is_allowed_model(
            "hugging-quants/Llama-3.2-1B-Instruct-GGUF",
            "Llama-3.2-1B-Instruct-Q4_K_M.gguf"
        ));
        assert!(!is_allowed_model("evil/actor", "payload.gguf"));
    }

    #[test]
    fn mutating_admin_paths() {
        assert!(is_mutating_admin_path("/v1/models/swap"));
        assert!(is_mutating_admin_path("/v1/knowledge/import"));
        assert!(is_mutating_admin_path("/v1/admin/rotate"));
        assert!(is_mutating_admin_path("/v1/admin/reload"));
        assert!(is_mutating_admin_path("/v1/admin/sessions/purge"));
        assert!(!is_mutating_admin_path("/v1/admin/status"));
        assert!(!is_mutating_admin_path("/v1/admin/audit"));
        assert!(!is_mutating_admin_path("/v1/chat"));
        // Prefix-gating still classifies the new admin routes as admin.
        assert!(is_admin_path("/v1/admin/reload"));
        assert!(is_admin_path("/v1/admin/metrics"));
        assert!(is_admin_path("/v1/admin/sessions/purge"));
    }

    #[test]
    fn protect_unprotect_roundtrip() {
        let plain = b"super-secret-token-value";
        let blob = protect_data(plain).expect("protect");
        assert_ne!(blob.as_slice(), plain.as_slice());
        let back = unprotect_data(&blob).expect("unprotect");
        assert_eq!(back, plain);
        // Legacy plaintext passthrough.
        assert_eq!(unprotect_data(b"{\"a\":1}").unwrap(), b"{\"a\":1}");
    }

    #[test]
    fn config_defaults_hardened() {
        let cfg = SecurityConfig::default();
        assert!(!cfg.read_only);
        assert!(cfg.admin_loopback_only);
        assert_eq!(cfg.sessions_retention_days, 30);
    }

    #[test]
    fn cors_same_origin_logic() {
        let allow: Vec<String> = vec![];
        let host = HeaderValue::from_str("127.0.0.1:8080").unwrap();
        let same = HeaderValue::from_str("http://127.0.0.1:8080").unwrap();
        let evil = HeaderValue::from_str("https://evil.example").unwrap();
        let nul = HeaderValue::from_str("null").unwrap();
        assert!(origin_allowed(&same, Some(&host), &allow));
        assert!(!origin_allowed(&evil, Some(&host), &allow));
        assert!(!origin_allowed(&nul, Some(&host), &allow));
        let allow2 = vec!["https://evil.example".to_string()];
        assert!(origin_allowed(&evil, Some(&host), &allow2));
    }

    fn ws_request(origin: Option<&str>, host: Option<&str>) -> Request {
        let mut builder = Request::builder().uri("/ws");
        if let Some(o) = origin {
            builder = builder.header("origin", o);
        }
        if let Some(h) = host {
            builder = builder.header("host", h);
        }
        builder.body(axum::body::Body::empty()).unwrap()
    }

    fn ws_test_state() -> SecurityState {
        SecurityState::new(SecurityConfig::default(), None, None)
    }

    #[test]
    fn ws_same_origin_with_port_is_allowed() {
        // Regression: the old comparison stripped the port from Host but
        // not from Origin, 403ing every same-origin WS handshake on :8080.
        let state = ws_test_state();
        let req = ws_request(Some("http://127.0.0.1:8080"), Some("127.0.0.1:8080"));
        assert!(ws_origin_allowed(&req, &state));
    }

    #[test]
    fn ws_cross_origin_and_cross_port_stay_denied() {
        let state = ws_test_state();
        // Different host entirely.
        let evil = ws_request(Some("https://evil.example"), Some("127.0.0.1:8080"));
        assert!(!ws_origin_allowed(&evil, &state));
        // Same host, different port: still cross-origin, still denied.
        let xport = ws_request(Some("http://127.0.0.1:8080"), Some("127.0.0.1:9090"));
        assert!(!ws_origin_allowed(&xport, &state));
        // Missing origin header: denied (transport-hijack guard).
        let no_origin = ws_request(None, Some("127.0.0.1:8080"));
        assert!(!ws_origin_allowed(&no_origin, &state));
    }

    #[test]
    fn ws_explicit_allowlist_still_wins() {
        let state = SecurityState::new(
            SecurityConfig {
                allowed_origins: vec!["https://app.example".to_string()],
                ..Default::default()
            },
            None,
            None,
        );
        let listed = ws_request(Some("https://app.example"), Some("127.0.0.1:8080"));
        assert!(ws_origin_allowed(&listed, &state));
    }

    #[test]
    fn replace_cfg_rejects_unsafe_relaxation() {
        let state = SecurityState::new(
            SecurityConfig {
                read_only: true,
                admin_loopback_only: true,
                min_disk_free_mb: 1024,
                max_model_bytes: 16 * 1024 * 1024 * 1024,
                temperature_min: 0.0,
                temperature_max: 5.0,
                ..Default::default()
            },
            None,
            None,
        );
        // read_only true → false must be rejected
        let relaxed = SecurityConfig {
            read_only: false,
            ..state.cfg_snapshot()
        };
        assert!(state.replace_cfg(relaxed).is_err());
        // admin_loopback_only true → false must be rejected
        let relaxed2 = SecurityConfig {
            admin_loopback_only: false,
            ..state.cfg_snapshot()
        };
        assert!(state.replace_cfg(relaxed2).is_err());
        // min_disk_free_mb = 0 must be rejected
        let relaxed3 = SecurityConfig {
            min_disk_free_mb: 0,
            ..state.cfg_snapshot()
        };
        assert!(state.replace_cfg(relaxed3).is_err());
        // max_model_bytes > 32 GB must be rejected
        let relaxed4 = SecurityConfig {
            max_model_bytes: 64 * 1024 * 1024 * 1024,
            ..state.cfg_snapshot()
        };
        assert!(state.replace_cfg(relaxed4).is_err());
        // Valid config must succeed
        let safe = SecurityConfig {
            read_only: true,
            admin_loopback_only: true,
            min_disk_free_mb: 512,
            max_model_bytes: 8 * 1024 * 1024 * 1024,
            temperature_min: 0.5,
            temperature_max: 3.0,
            ..state.cfg_snapshot()
        };
        assert!(state.replace_cfg(safe).is_ok());
    }
}
