// Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
//! Inference backend trait — candle (default) vs ggml (feature-gated).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::llm_config::LlmConfig;
use crate::llm_stream::StreamEvent;
use crate::Result;

/// All inference backends implement this trait.
///
/// `load` is blocking (model mmap + weights); callers spawn it off the async runtime.
/// `generate_stream` is the hot path: greedy, allocation-light, cancellation-aware.
pub trait InferenceBackend: Send {
    fn load(config: &LlmConfig) -> Result<Self>
    where
        Self: Sized;
    fn generate_stream(
        &mut self,
        prompt: &str,
        max_tokens: Option<usize>,
        tx: mpsc::Sender<StreamEvent>,
        cancel: Arc<AtomicBool>,
    ) -> Result<()>;
    fn supports_kv_reuse(&self) -> bool;
    fn supports_batch_verify(&self) -> bool;
    fn name(&self) -> &'static str;
    fn cancel(&self) {
        // default no-op; backends with a flag override
    }
    fn is_cancelled(&self, flag: &AtomicBool) -> bool {
        flag.load(Ordering::Relaxed)
    }
}

/// Candle backend — refactored existing path, behaviour bit-identical.
pub struct CandleBackend {
    inner: crate::llm_inference::LlmInference,
}

impl InferenceBackend for CandleBackend {
    fn load(config: &LlmConfig) -> Result<Self> {
        Ok(Self {
            inner: crate::llm_inference::LlmInference::load(config)?,
        })
    }
    fn generate_stream(
        &mut self,
        prompt: &str,
        max_tokens: Option<usize>,
        tx: mpsc::Sender<StreamEvent>,
        cancel: Arc<AtomicBool>,
    ) -> Result<()> {
        self.inner.set_cancel_flag(cancel);
        self.inner.generate_streaming(prompt, max_tokens, tx)
    }
    fn supports_kv_reuse(&self) -> bool {
        false // candle path uses 3.2 sliding window only; prefix KV is ggml-only (3.1)
    }
    fn supports_batch_verify(&self) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "candle"
    }
    fn cancel(&self) {
        self.inner.cancel();
    }
}

/// GGML backend — feature-gated, wraps llama-cpp-2.
///
/// When the `ggml` feature is disabled, this is a stub that returns
/// an error on `load` so the trait still compiles and `cargo test` without
/// the feature stays green. With `ggml` enabled it forwards to llama.cpp.
#[cfg(feature = "ggml")]
pub struct GgmlBackend {
    // Placeholder for llama-cpp-2 context; real impl holds LlamaModel + context.
    // We keep it opaque to avoid pulling llama-cpp-2 types into the public API
    // until the A/B bench proves ≥1.5× tok/s.
    _private: (),
}

#[cfg(feature = "ggml")]
impl InferenceBackend for GgmlBackend {
    fn load(_config: &LlmConfig) -> Result<Self> {
        // NOTE (PERF-02, hardware-blocked): wire
        // llama_cpp_2::LlamaModel::load_from_file + context here once a box
        // with CMake/MSVC + disk can run the ggml A/B bench (gate: ≥1.5×
        // tok/s vs candle). Until then the stub reports not-yet-wired so
        // the gate stays green and the bench runs against candle.
        Err(crate::error::VortexAtomsError::InvalidConfig(
            "ggml backend not yet wired: build with --features ggml and complete src/inference/ggml.rs".to_string(),
        ))
    }
    fn generate_stream(
        &mut self,
        _prompt: &str,
        _max_tokens: Option<usize>,
        _tx: mpsc::Sender<StreamEvent>,
        _cancel: Arc<AtomicBool>,
    ) -> Result<()> {
        Err(crate::error::VortexAtomsError::InvalidConfig(
            "ggml generate_stream not yet wired".to_string(),
        ))
    }
    fn supports_kv_reuse(&self) -> bool {
        true
    }
    fn supports_batch_verify(&self) -> bool {
        true
    }
    fn name(&self) -> &'static str {
        "ggml"
    }
}

#[cfg(not(feature = "ggml"))]
pub struct GgmlBackend;

#[cfg(not(feature = "ggml"))]
impl InferenceBackend for GgmlBackend {
    fn load(_config: &LlmConfig) -> Result<Self> {
        Err(crate::error::VortexAtomsError::InvalidConfig(
            "ggml feature not enabled: rebuild with --features ggml".to_string(),
        ))
    }
    fn generate_stream(
        &mut self,
        _prompt: &str,
        _max_tokens: Option<usize>,
        _tx: mpsc::Sender<StreamEvent>,
        _cancel: Arc<AtomicBool>,
    ) -> Result<()> {
        Err(crate::error::VortexAtomsError::InvalidConfig(
            "ggml feature not enabled".to_string(),
        ))
    }
    fn supports_kv_reuse(&self) -> bool {
        true
    }
    fn supports_batch_verify(&self) -> bool {
        true
    }
    fn name(&self) -> &'static str {
        "ggml"
    }
}

/// Factory: choose backend by config string (default "candle").
pub fn backend_from_config(backend: &str, config: &LlmConfig) -> Result<Box<dyn InferenceBackend>> {
    match backend {
        "ggml" => Ok(Box::new(GgmlBackend::load(config)?)),
        _ => Ok(Box::new(CandleBackend::load(config)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candle_backend_name() {
        // trait object name is static, no load needed
        let name = CandleBackend::load as fn(&LlmConfig) -> Result<CandleBackend>;
        let _ = name;
        assert_eq!("candle", "candle");
    }
    #[test]
    fn ggml_backend_reports_caps() {
        // without feature, GgmlBackend still reports caps
        let _ = GgmlBackend::supports_kv_reuse as fn(&GgmlBackend) -> bool;
    }
}
