// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//! Per-session inference state: conversation history + sampler + cancel.
//!
//! The model weights stay shared and serial inside one `LlmInference`
//! (a Qwen2 `ModelWeights` is not `Clone`, and candle keeps mutable KV
//! state inside the model struct, so true engine clones are impossible
//! without restructuring vendor types). Sessions partition everything
//! that must not leak across clients:
//!
//! - `history`: per-client conversation; the engine's own history vec is
//!   never read while a session is swapped in.
//! - `sampler`: per-client temperature + RNG stream. Previously
//!   `set_temperature` mutated the shared engine sampler and persisted
//!   across unrelated requests; now it applies to the session only.
//! - `cancel_flag`: per-session cancel. `generate_with_session` installs
//!   this flag on the engine for the call only, so cancelling one session
//!   never bleeds into the engine default or another session.
//!
//! Weight storage stays shared: candle tensors are immutable, so cloned
//! handles cannot alias mutably. KV state is rebuilt per request under
//! the serial engine lock (`cache.reset` → `model.clear_kv_cache`), and
//! the shared prefix cache is hash-validated. Per-session RAM is KBs
//! (history + sampler); no weight duplication, ever.
//!
//! Ownership: WS connections own one session for their lifetime; stateless
//! HTTP endpoints use an ephemeral session per request; IKC / tool / batch
//! / agent paths fork an ephemeral session per call (they previously ran
//! bare-engine `set_temperature` + `generate` and leaked temperature and
//! history across requests). Residual sharing (documented): n-gram drafter
//! tables live on the engine's single `SpeculativeDecoder` (shared across
//! sessions — see `speculative_enabled`); verification is exact
//! (Levi / argmax), so shared draft tables cannot skew a session's output
//! distribution. Also shared: the hash-validated prefix-KV cache and the
//! engine-resident cancel flag used only by bare-engine callers (CLI,
//! backend trait) — session paths never touch it.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::llm_config::SamplingConfig;
use crate::llm_prompt::{ChatMessage, MessageRole};
use crate::llm_sampling::VortexSampler;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

/// Inference state private to one client session.
pub struct InferenceSession {
    /// Unique per process (monotonic counter; never reused).
    pub id: u64,
    /// This session's conversation only. Pushed by generate paths while
    /// the session is swapped into the engine.
    pub history: Vec<ChatMessage>,
    /// This session's sampler (temperature + RNG stream).
    pub sampler: VortexSampler,
    /// Session-scoped cancel flag. Installed on the engine only while a
    /// `*_with_session` generate runs; never the engine default.
    pub cancel_flag: Arc<AtomicBool>,
}

impl InferenceSession {
    /// Fresh session: empty history, sampler forked from `(seed, config)`
    /// with the id mixed in so concurrent sessions never share an RNG stream.
    pub fn new(seed: u64, config: &SamplingConfig) -> Self {
        let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let sampler = VortexSampler::new(seed.wrapping_add(id), config);
        Self {
            id,
            history: Vec::new(),
            sampler,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Session-scoped temperature: sticky for this session only, invisible
    /// to every other session and to the engine defaults.
    pub fn set_temperature(&mut self, temp: f64) {
        self.sampler.set_temperature(temp);
    }

    /// Request cancel for this session only (takes effect on the next
    /// check inside an active `*_with_session` generate).
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    /// Clear this session's conversation (batch steps / agent steps that
    /// must not observe prior turns).
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Record a turn in this session's history (mirrors the engine's
    /// end-of-generate bookkeeping shape).
    pub fn push_turn(&mut self, user: String, assistant: String) {
        self.history.push(ChatMessage {
            role: MessageRole::User,
            content: user,
        });
        self.history.push(ChatMessage {
            role: MessageRole::Assistant,
            content: assistant,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SamplingConfig {
        SamplingConfig {
            temperature: None,
            top_k: None,
            top_p: None,
            repeat_penalty: 1.1,
            repeat_last_n: 64,
        }
    }

    #[test]
    fn test_session_ids_are_unique_and_monotonic() {
        let cfg = test_config();
        let a = InferenceSession::new(42, &cfg);
        let b = InferenceSession::new(42, &cfg);
        assert_ne!(a.id, b.id);
        assert!(b.id > a.id);
    }

    #[test]
    fn test_session_starts_empty_and_greedy_by_default() {
        let cfg = test_config();
        let s = InferenceSession::new(7, &cfg);
        assert!(s.history.is_empty());
        assert!(s.sampler.is_greedy());
    }

    #[test]
    fn test_session_temperature_is_sticky_and_isolated() {
        let cfg = test_config();
        let mut a = InferenceSession::new(1, &cfg);
        let mut b = InferenceSession::new(1, &cfg);
        a.set_temperature(0.9);
        assert!(!a.sampler.is_greedy());
        // Same seed, other session untouched: isolation is structural.
        assert!(b.sampler.is_greedy());
        b.set_temperature(0.0);
        assert!(b.sampler.is_greedy());
        assert!(!a.sampler.is_greedy());
    }

    #[test]
    fn test_session_history_does_not_leak() {
        let cfg = test_config();
        let mut a = InferenceSession::new(1, &cfg);
        let b = InferenceSession::new(1, &cfg);
        a.push_turn("hello".into(), "hi".into());
        assert_eq!(a.history.len(), 2);
        assert!(b.history.is_empty());
    }

    #[test]
    fn test_session_cancel_is_isolated() {
        let cfg = test_config();
        let a = InferenceSession::new(1, &cfg);
        let b = InferenceSession::new(1, &cfg);
        assert!(!a.is_cancelled());
        a.cancel();
        assert!(a.is_cancelled());
        assert!(!b.is_cancelled());
        b.cancel();
        assert!(b.is_cancelled());
        a.cancel_flag.store(false, Ordering::Relaxed);
        assert!(!a.is_cancelled());
        assert!(b.is_cancelled());
    }

    #[test]
    fn test_session_clear_history_only_clears_self() {
        let cfg = test_config();
        let mut a = InferenceSession::new(1, &cfg);
        let mut b = InferenceSession::new(1, &cfg);
        a.push_turn("u".into(), "a".into());
        b.push_turn("u2".into(), "a2".into());
        a.clear_history();
        assert!(a.history.is_empty());
        assert_eq!(b.history.len(), 2);
    }
}
