use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use memmap2::Mmap;
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokenizers::Tokenizer;
use tokio::sync::mpsc;

use crate::build_info;
use crate::error::VortexAtomsError;
use crate::llm_cache::VortexCache;
use crate::llm_config::{LlmConfig, ModelArchitecture};
use crate::llm_model::{
    detect_architecture, detect_eos_token_id, load_gguf_model, LlmModel, LlmModelEnum,
};
use crate::llm_prompt::{format_chat_history, ChatMessage, ChatTemplate, MessageRole};
use crate::llm_sampling::VortexSampler;
use crate::llm_stream::{StreamEvent, TokenOutputStream};
use crate::perf_topology;
use crate::Result;

const DEFAULT_MAX_HISTORY_TURNS: usize = 20;

pub struct LlmInference {
    model: LlmModelEnum,
    tokenizer: Tokenizer,
    sampler: VortexSampler,
    eos_token_id: u32,
    config: LlmConfig,
    template: ChatTemplate,
    cache: VortexCache,
    cancel_flag: Arc<AtomicBool>,
    conversation_history: Vec<ChatMessage>,
    max_history_turns: usize,
    simd: String,
    device: Device,
    /// Optional memory map of the model file for prefaulting.
    mmap: Option<Mmap>,
    /// Reused per-token buffers — zero per-token allocs (1.2).
    /// `logits_scratch` holds dequantized logits (vocab ~32k, reused).
    /// `input_scratch` is not needed (candle Tensor is ephemeral) but we keep
    /// the Vec<u32> token window reused via `all_tokens` capacity.
    logits_scratch: Vec<f32>,
    /// Target distribution buffer for probabilistic speculative acceptance.
    prob_scratch: Vec<f32>,
    /// Layer double-buffer prefetch toggle (1.4, config `layer_prefetch`).
    layer_prefetch: bool,
    /// Sliding window KV cache size (config `kv_cache_window`).
    kv_cache_window: Option<usize>,
    /// Static prefix KV cache enabled (config `kv_prefix_cache`).
    kv_prefix_cache: bool,
    /// Cached prefix tokens for prefix KV reuse.
    cached_prefix_tokens: Option<Vec<u32>>,
    /// Speculative decoding coordinator (drafter + Levi/greedy verifier).
    speculative_decoder: Option<crate::speculative::SpeculativeDecoder>,
    /// Speculative decoding enabled (config `speculative_enabled`).
    speculative_enabled: bool,
}

impl LlmInference {
    pub fn load(config: &LlmConfig) -> Result<Self> {
        let device = config.device_type.to_device();

        let file = File::open(&config.model_path).map_err(VortexAtomsError::Io)?;
        let mut buf_reader = BufReader::new(file);
        let content = gguf_file::Content::read(&mut buf_reader)
            .map_err(|e| VortexAtomsError::Gguf(e.to_string()))?;

        let arch = match &config.architecture {
            Some(a) => a.clone(),
            None => detect_architecture(&content)?,
        };

        let eos_token_id = detect_eos_token_id(&content);

        // Create memory map for prefaulting (done before model load so we can
        // start prefaulting early without blocking the health endpoint).
        let mmap_file = File::open(&config.model_path).map_err(VortexAtomsError::Io)?;
        let mmap = unsafe { Mmap::map(&mmap_file) }.map_err(VortexAtomsError::Io)?;

        let file_for_model = File::open(&config.model_path).map_err(VortexAtomsError::Io)?;
        let mut buf_reader_for_model = BufReader::new(file_for_model);
        let model = load_gguf_model(
            content,
            &mut buf_reader_for_model,
            &device,
            &arch,
            config.use_flash_attn,
        )?;

        let tokenizer =
            Tokenizer::from_file(&config.tokenizer_path).map_err(VortexAtomsError::Tokenizers)?;

        let sampler = VortexSampler::new(config.seed, &config.sampling);
        let template = ChatTemplate::default_for_arch(&arch.to_string());

        let perf = crate::vortex_config::VortexConfig::load().performance;
        let layer_prefetch = perf.layer_prefetch;
        let kv_cache_window = perf.kv_cache_window;
        let kv_prefix_cache = perf.kv_prefix_cache;
        let speculative_enabled = perf.speculative_enabled;
        let max_draft_tokens = perf.max_draft_tokens;
        let min_acceptance_rate = perf.min_acceptance_rate;
        let ngram_order = perf.ngram_order;

        let cache = VortexCache::with_config(config.max_seq_len, kv_cache_window, kv_prefix_cache);

        // Initialize speculative decoding if enabled (single drafter instance:
        // the decoder owns the only NGramDrafter — no divergent twin tables).
        let speculative_decoder = if speculative_enabled {
            let drafter = crate::speculative::NGramDrafter::new(
                ngram_order,
                32000, // vocab size
                config.sampling.temperature.unwrap_or(0.8) as f32,
                config.seed,
            );
            Some(crate::speculative::SpeculativeDecoder::new_seeded(
                Box::new(drafter),
                max_draft_tokens,
                min_acceptance_rate,
                config.seed ^ 0x5EED_1A7E,
            ))
        } else {
            None
        };

        let simd = build_info::simd_summary();

        let inference = Self {
            model,
            tokenizer,
            sampler,
            eos_token_id,
            config: config.clone(),
            template,
            cache,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            conversation_history: Vec::new(),
            max_history_turns: DEFAULT_MAX_HISTORY_TURNS,
            simd,
            device,
            mmap: Some(mmap),
            logits_scratch: Vec::with_capacity(32000),
            prob_scratch: Vec::with_capacity(32000),
            layer_prefetch,
            kv_cache_window,
            kv_prefix_cache,
            cached_prefix_tokens: None,
            speculative_decoder,
            speculative_enabled,
        };

        // Spawn background prefault if configured (non-blocking).
        if config.prefault {
            if let Some(ref mmap) = inference.mmap {
                println!("[API] Prefault: background thread started (1 MiB stride)");
                let _handle = perf_topology::spawn_prefault(mmap, 1024 * 1024);
            }
        }

        Ok(inference)
    }

    pub fn set_cancel_flag(&mut self, flag: Arc<AtomicBool>) {
        self.cancel_flag = flag;
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn tokenize(&self, text: &str) -> Result<Vec<u32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(VortexAtomsError::Tokenizers)?;
        Ok(encoding.get_ids().to_vec())
    }

    pub fn decode_tokens(&self, tokens: &[u32]) -> Result<String> {
        let text = self
            .tokenizer
            .decode(tokens, true)
            .map_err(VortexAtomsError::Tokenizers)?;
        Ok(text)
    }

    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    /// Fork a pristine session from this engine's config (see `inference_session`).
    /// Shares nothing mutable: fresh history, sampler rebuilt from
    /// `(seed, sampling)` with the session id salted in. Weights stay
    /// shared; KV state is rebuilt per request under the serial lock.
    pub fn fork_session(&self) -> crate::inference_session::InferenceSession {
        crate::inference_session::InferenceSession::new(self.config.seed, &self.config.sampling)
    }

    /// Generate with a session swapped in: the session's history, sampler,
    /// and cancel flag replace the engine-resident ones for the call, then
    /// swap back — including on `Err` (result is captured first). Callers
    /// must hold the engine exclusively; all do (the write guard spans
    /// blocking compute). Weights and KV/prefix caches stay shared by
    /// design: KV is rebuilt per request and the prefix cache is
    /// hash-validated. Session cancel never touches the engine default flag.
    pub fn generate_with_session(
        &mut self,
        user_message: &str,
        max_tokens: Option<usize>,
        session: &mut crate::inference_session::InferenceSession,
    ) -> Result<String> {
        std::mem::swap(&mut self.conversation_history, &mut session.history);
        std::mem::swap(&mut self.sampler, &mut session.sampler);
        let engine_flag =
            std::mem::replace(&mut self.cancel_flag, Arc::clone(&session.cancel_flag));
        let out = self.generate(user_message, max_tokens);
        self.cancel_flag = engine_flag;
        std::mem::swap(&mut self.conversation_history, &mut session.history);
        std::mem::swap(&mut self.sampler, &mut session.sampler);
        out
    }

    /// Streaming twin of `generate_with_session`: same swap discipline.
    pub fn generate_streaming_with_session(
        &mut self,
        user_message: &str,
        max_tokens: Option<usize>,
        tx: mpsc::Sender<crate::llm_stream::StreamEvent>,
        session: &mut crate::inference_session::InferenceSession,
    ) -> Result<()> {
        std::mem::swap(&mut self.conversation_history, &mut session.history);
        std::mem::swap(&mut self.sampler, &mut session.sampler);
        let engine_flag =
            std::mem::replace(&mut self.cancel_flag, Arc::clone(&session.cancel_flag));
        let out = self.generate_streaming(user_message, max_tokens, tx);
        self.cancel_flag = engine_flag;
        std::mem::swap(&mut self.conversation_history, &mut session.history);
        std::mem::swap(&mut self.sampler, &mut session.sampler);
        out
    }

    /// Session-isolated batch: each prompt runs on the session with a
    /// cleared session history so entries cannot observe each other (or
    /// pollute engine-resident history).
    pub fn batch_generate_with_session(
        &mut self,
        prompts: &[&str],
        max_tokens: Option<usize>,
        session: &mut crate::inference_session::InferenceSession,
    ) -> Result<Vec<String>> {
        let mut results = Vec::with_capacity(prompts.len());
        for prompt in prompts {
            session.clear_history();
            results.push(self.generate_with_session(prompt, max_tokens, session)?);
        }
        Ok(results)
    }

    pub fn append_to_history(&mut self, role: MessageRole, content: &str) {
        self.conversation_history.push(ChatMessage {
            role,
            content: content.to_string(),
        });
    }

    pub fn set_max_history_turns(&mut self, max: usize) {
        self.max_history_turns = max;
    }

    pub fn conversation_history(&self) -> &[ChatMessage] {
        &self.conversation_history
    }

    fn build_messages_with_context(
        &self,
        user_message: &str,
        rag_context: Option<&str>,
    ) -> Vec<ChatMessage> {
        let mut messages = Vec::new();
        messages.push(ChatMessage {
            role: MessageRole::System,
            content: self.config.system_prompt.clone(),
        });

        for msg in &self.conversation_history {
            messages.push(msg.clone());
        }

        let effective_message = if let Some(context) = rag_context {
            format!(
                "Use the following context to answer the question.\n\nContext:\n{}\n\nQuestion: {}",
                context, user_message
            )
        } else {
            user_message.to_string()
        };

        messages.push(ChatMessage {
            role: MessageRole::User,
            content: effective_message,
        });

        messages
    }

    /// Extract the system prompt prefix tokens for prefix KV caching.
    /// Returns the tokenized system prompt (and optionally conversation history prefix).
    fn extract_system_prefix_tokens(&self, messages: &[ChatMessage]) -> Vec<u32> {
        if messages.is_empty() {
            return Vec::new();
        }
        // The first message is always the system prompt.
        // Tokenize just the system prompt for prefix KV caching.
        let system_msg = &messages[0];
        let system_text =
            format_chat_history(&self.template, std::slice::from_ref(system_msg), None);
        self.tokenize(&system_text).unwrap_or_default()
    }

    /// Get the cached prefix tokens if available.
    pub fn get_cached_prefix(&self) -> Option<&[u32]> {
        self.cached_prefix_tokens.as_deref()
    }

    /// Train the decoder-owned n-gram drafter on history + system + prompt.
    fn train_speculative_drafter(&mut self, prompt_tokens: &[u32]) {
        if !self.speculative_enabled {
            return;
        }
        let mut train_buf: Vec<u32> = Vec::with_capacity(prompt_tokens.len() * 2);
        for msg in &self.conversation_history {
            if let Ok(t) = self.tokenize(&msg.content) {
                train_buf.extend(t);
            }
        }
        if let Ok(t) = self.tokenize(&self.config.system_prompt) {
            train_buf.extend(t);
        }
        train_buf.extend_from_slice(prompt_tokens);
        if let Some(decoder) = &mut self.speculative_decoder {
            if !train_buf.is_empty() {
                decoder.drafter.update(&train_buf);
            }
        }
    }

    /// One speculative generation step (Levi probabilistic or greedy).
    ///
    /// Returns `(tokens_to_commit, forwards_performed)`.
    ///
    /// KV contract (mirrors the normal decode loop exactly):
    /// - On entry, `context` is `all_tokens` and `pos == context.len()`.
    ///   The normal path would `forward(context.last(), pos)` → sample →
    ///   `push` → `pos += 1`.
    /// - We perform that same first forward (p₀ verifies draft[0]).
    /// - On accept of draft i we forward that token (as the normal path
    ///   would forward the token it just sampled) to obtain pᵢ₊₁.
    /// - On reject we emit a residual/argmax replacement and stop
    ///   **without** forwarding the rejected draft or the replacement —
    ///   the next loop iteration forwards the replacement, exactly like
    ///   the normal path forwards a freshly sampled token.
    /// - On full accept we forward the last accepted draft for bonus
    ///   logits, sample the bonus, and commit it unforwarded (same lag).
    /// - EOS stops the step immediately (not forwarded).
    ///
    /// Invariant: `forwards_performed == tokens_to_commit.len()`, so the
    /// caller does `pos += forwards_performed` and KV stays aligned with
    /// the normal `forward → sample → push → pos += 1` cadence.
    fn speculative_step(&mut self, context: &[u32], max_draft: usize) -> Result<(Vec<u32>, usize)> {
        if !self.speculative_enabled || context.is_empty() || max_draft == 0 {
            return Ok((Vec::new(), 0));
        }
        let greedy = self.sampler.is_greedy();
        let eos = self.eos_token_id;

        let proposals = match &mut self.speculative_decoder {
            Some(dec) => dec.draft_with_probs(context, max_draft),
            None => return Ok((Vec::new(), 0)),
        };
        if proposals.is_empty() {
            return Ok((Vec::new(), 0));
        }
        let drafted: Vec<u32> = proposals.iter().map(|(t, _)| *t).collect();
        let qs: Vec<crate::speculative::SparseDist> =
            proposals.iter().map(|(_, q)| q.clone()).collect();

        // Forward context.last() at pos=context.len() → p₀ (verifies draft[0]).
        let pos = context.len();
        let mut logits_vec = {
            let input = Tensor::new(&[*context.last().unwrap()], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?;
            logits.to_vec1::<f32>()?
        };
        let mut forwards = 1usize;

        let mut penalty_ctx = context.to_vec();
        let mut committed: Vec<u32> = Vec::with_capacity(drafted.len() + 1);
        let mut hit_eos = false;
        let mut rejected = false;

        for (i, &d) in drafted.iter().enumerate() {
            let (token, accepted) = if greedy {
                // Same penalty window as sample_with_scratch, then argmax.
                self.sampler.penalized_logits_into(
                    &logits_vec,
                    &penalty_ctx,
                    &mut self.prob_scratch,
                );
                let dec = self.speculative_decoder.as_mut().unwrap();
                dec.accept_greedy(d, &self.prob_scratch)
            } else {
                self.sampler.target_probs_logits(
                    &logits_vec,
                    &penalty_ctx,
                    &mut self.prob_scratch,
                )?;
                let dec = self.speculative_decoder.as_mut().unwrap();
                let p = std::mem::take(&mut self.prob_scratch);
                let r = dec.accept_levi(d, &qs[i], &p);
                self.prob_scratch = p;
                r
            };

            committed.push(token);
            penalty_ctx.push(token);

            if token == eos {
                hit_eos = true;
                break;
            }
            if !accepted {
                rejected = true;
                break;
            }

            // Accepted: forward this token to get logits for the next draft
            // (or, on the last draft, the bonus-token logits).
            let input = Tensor::new(&[token], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, pos + forwards)?;
            let logits = logits.squeeze(0)?.squeeze(0)?;
            logits_vec = logits.to_vec1::<f32>()?;
            forwards += 1;
        }

        if !hit_eos && !rejected && !committed.is_empty() {
            // logits_vec is the bonus distribution (after last accepted draft).
            let bonus = if greedy {
                self.sampler.penalized_logits_into(
                    &logits_vec,
                    &penalty_ctx,
                    &mut self.prob_scratch,
                );
                crate::llm_sampling::argmax_index(&self.prob_scratch) as u32
            } else {
                self.sampler.target_probs_logits(
                    &logits_vec,
                    &penalty_ctx,
                    &mut self.prob_scratch,
                )?;
                let u = match &mut self.speculative_decoder {
                    Some(dec) => dec.random_f32(),
                    None => 0.0,
                };
                let p = &self.prob_scratch;
                let mut r = u;
                let mut chosen = crate::llm_sampling::argmax_index(p) as u32;
                for (idx, &pi) in p.iter().enumerate() {
                    r -= pi;
                    if r <= 0.0 {
                        chosen = idx as u32;
                        break;
                    }
                }
                chosen
            };
            committed.push(bonus);
            // bonus is NOT forwarded (normal-path lag).
            if bonus == eos {
                hit_eos = true;
            }
        }

        // Teach the drafter what stuck (drop a trailing rejection replacement).
        if let Some(dec) = &mut self.speculative_decoder {
            let n_ok = if rejected || hit_eos {
                // Last committed may be a replacement or EOS; EOS was a real
                // draft/sample so count it; a rejection replacement should not
                // train as an accepted draft.
                if rejected {
                    committed.len().saturating_sub(1)
                } else {
                    committed.len()
                }
            } else {
                committed.len() // all accepted + bonus
            };
            if n_ok > 0 {
                dec.drafter.update(&committed[..n_ok]);
            }
            // Step-level acceptance gate: caller may consult this via
            // `acceptance_rate()`; `min_acceptance_rate` lives on the decoder.
        }

        debug_assert_eq!(
            forwards,
            committed.len(),
            "KV forward count must match committed tokens"
        );

        Ok((committed, forwards))
    }

    pub fn generate(&mut self, user_message: &str, max_tokens: Option<usize>) -> Result<String> {
        self.generate_with_rag(user_message, max_tokens, None)
    }

    pub fn generate_with_rag(
        &mut self,
        user_message: &str,
        max_tokens: Option<usize>,
        rag_context: Option<&str>,
    ) -> Result<String> {
        self.cancel_flag.store(false, Ordering::Relaxed);

        let messages = self.build_messages_with_context(user_message, rag_context);
        let prompt = format_chat_history(&self.template, &messages, None);
        let tokens = self.tokenize(&prompt)?;
        let max = max_tokens.unwrap_or(self.config.max_generation_tokens);

        if tokens.is_empty() {
            return Ok(String::new());
        }

        // Determine if we can reuse a cached prefix KV.
        let prefix_tokens = self.extract_system_prefix_tokens(&messages);
        let use_prefix_kv = self.kv_prefix_cache
            && !prefix_tokens.is_empty()
            && self.cache.has_prefix_kv(&prefix_tokens);

        // Reset cache, preserving prefix KV if available.
        if use_prefix_kv {
            self.cache
                .reset_with_prefix(&mut self.model, &prefix_tokens);
            self.cached_prefix_tokens = Some(prefix_tokens.clone());
        } else {
            self.cache.reset(&mut self.model, None::<&[u32]>);
            self.cached_prefix_tokens = None;
        }

        // Pre-allocate token window to avoid Vec growth inside the hot loop (1.2).
        let mut all_tokens = Vec::with_capacity(tokens.len() + max);
        all_tokens.extend_from_slice(&tokens);
        let mut pos = 0;

        // Prefill: reuse same Tensor path, no per-token String.
        for &token in &tokens {
            if self.layer_prefetch {
                if let Some(ref mmap) = self.mmap {
                    crate::perf_topology::prefetch_next_layer(mmap, pos * 4096, 1 << 20);
                }
            }
            let input = Tensor::new(&[token], &self.device)?.unsqueeze(0)?;
            let _logits = self.model.forward(&input, pos)?;
            pos += 1;
        }

        // Store prefix KV after prefill if prefix caching enabled and this is a new prefix.
        if self.kv_prefix_cache && !use_prefix_kv && !prefix_tokens.is_empty() {
            self.cache
                .store_prefix_kv(&prefix_tokens, prefix_tokens.len());
            self.cached_prefix_tokens = Some(prefix_tokens);
        }

        let start = Instant::now();
        let mut generated = 0usize;
        // Ensure logits scratch has vocab capacity once (reuse, no per-token alloc).
        if self.logits_scratch.capacity() < 32000 {
            self.logits_scratch
                .reserve(32000 - self.logits_scratch.capacity());
        }

        // Train the n-gram drafter on prompt + history before drafting.
        self.train_speculative_drafter(&tokens);
        let mut spec_allow = self.speculative_enabled;

        while generated < max {
            if self.is_cancelled() {
                break;
            }
            // Layer double-buffer prefetch (1.4): hint next layer's pages while
            // current matmul runs. Config `layer_prefetch:true` default ON.
            if self.layer_prefetch {
                if let Some(ref mmap) = self.mmap {
                    let off = (pos * 4096) % mmap.len().max(1);
                    crate::perf_topology::prefetch_next_layer(mmap, off, 1 << 20);
                }
            }

            // Sliding window KV: if we exceed the window, shift the KV cache.
            if let Some(window) = self.kv_cache_window {
                if pos >= window {
                    // Shift KV cache: clear and re-feed last `window` tokens.
                    // This is a practical approximation since candle doesn't expose
                    // direct KV tensor shifting. We clear and replay the last window.
                    self.cache.reset(&mut self.model, None);
                    // Re-feed the last `window` tokens from all_tokens.
                    let replay_start = all_tokens.len().saturating_sub(window);
                    let replay_tokens = &all_tokens[replay_start..];
                    pos = 0;
                    for &token in replay_tokens {
                        if self.layer_prefetch {
                            if let Some(ref mmap) = self.mmap {
                                crate::perf_topology::prefetch_next_layer(
                                    mmap,
                                    pos * 4096,
                                    1 << 20,
                                );
                            }
                        }
                        let input = Tensor::new(&[token], &self.device)?.unsqueeze(0)?;
                        let _ = self.model.forward(&input, pos)?;
                        pos += 1;
                    }
                }
            }

            // Layer double-buffer prefetch (1.4): hint next layer's pages while
            // current matmul runs. Config `layer_prefetch:true` default ON.
            if self.layer_prefetch {
                if let Some(ref mmap) = self.mmap {
                    let off = (pos * 4096) % mmap.len().max(1);
                    crate::perf_topology::prefetch_next_layer(mmap, off, 1 << 20);
                }
            }

            // Speculative decoding: probabilistic (Levi) or greedy acceptance.
            // Enabled by `performance.speculative_enabled` (default off →
            // output distribution bit-identical to the normal path).
            // Greedy: argmax-exact. Sampling modes: Levi accept/reject with
            // residual resampling so the target distribution is preserved.
            // After a non-empty step the KV has already advanced — we must
            // `continue` (never fall through to a second forward).
            if self.speculative_enabled && spec_allow {
                let remaining = max - generated;
                let max_draft = self
                    .speculative_decoder
                    .as_ref()
                    .map(|d| d.max_draft_tokens)
                    .unwrap_or(0)
                    .min(remaining.saturating_sub(1));
                if max_draft > 0 {
                    if let Ok((committed, forwards)) = self.speculative_step(&all_tokens, max_draft)
                    {
                        if !committed.is_empty() {
                            debug_assert_eq!(forwards, committed.len());
                            debug_assert!(committed.len() <= remaining);
                            let mut hit_eos = false;
                            for &tok in &committed {
                                all_tokens.push(tok);
                                generated += 1;
                                pos += 1;
                                if tok == self.eos_token_id {
                                    hit_eos = true;
                                    break;
                                }
                            }
                            if hit_eos || generated >= max {
                                break;
                            }
                            // Pause speculation when the running acceptance
                            // rate falls below the decoder's threshold.
                            spec_allow = self
                                .speculative_decoder
                                .as_ref()
                                .map(|d| {
                                    d.total_drafted == 0
                                        || d.acceptance_rate() >= d.min_acceptance_rate
                                })
                                .unwrap_or(false);
                            continue;
                        }
                        // Empty draft → fall through to normal sampling.
                    }
                }
            }

            let last_token = *all_tokens.last().unwrap();
            // Single-element input Tensor: 1 alloc of 4 bytes, reused via capacity;
            // no String/format! inside loop (1.2).
            let input = Tensor::new(&[last_token], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?;

            // Greedy fast path is pure argmax (1.3) with reused scratch (1.2).
            let next_token =
                self.sampler
                    .sample_with_scratch(&logits, &all_tokens, &mut self.logits_scratch)?;
            all_tokens.push(next_token);
            generated += 1;
            pos += 1;

            if next_token == self.eos_token_id {
                break;
            }
        }

        let new_tokens = &all_tokens[tokens.len()..];
        let output = self.decode_tokens(new_tokens)?;

        let _elapsed = start.elapsed().as_secs_f64();

        self.conversation_history.push(ChatMessage {
            role: MessageRole::User,
            content: user_message.to_string(),
        });
        self.conversation_history.push(ChatMessage {
            role: MessageRole::Assistant,
            content: output.clone(),
        });

        self.trim_history();

        Ok(output)
    }

    pub fn generate_streaming(
        &mut self,
        user_message: &str,
        max_tokens: Option<usize>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<()> {
        self.generate_streaming_with_rag(user_message, max_tokens, tx, None)
    }

    pub fn generate_streaming_with_rag(
        &mut self,
        user_message: &str,
        max_tokens: Option<usize>,
        tx: mpsc::Sender<StreamEvent>,
        rag_context: Option<&str>,
    ) -> Result<()> {
        self.cancel_flag.store(false, Ordering::Relaxed);

        let messages = self.build_messages_with_context(user_message, rag_context);
        let prompt = format_chat_history(&self.template, &messages, None);
        let tokens = self.tokenize(&prompt)?;
        let max = max_tokens.unwrap_or(self.config.max_generation_tokens);

        if tokens.is_empty() {
            let _ = tx.blocking_send(StreamEvent::Error("Empty prompt".into()));
            return Ok(());
        }

        // Determine if we can reuse a cached prefix KV.
        let prefix_tokens = self.extract_system_prefix_tokens(&messages);
        let use_prefix_kv = self.kv_prefix_cache
            && !prefix_tokens.is_empty()
            && self.cache.has_prefix_kv(&prefix_tokens);

        // Reset cache, preserving prefix KV if available.
        if use_prefix_kv {
            self.cache
                .reset_with_prefix(&mut self.model, &prefix_tokens);
            self.cached_prefix_tokens = Some(prefix_tokens.clone());
        } else {
            self.cache.reset(&mut self.model, None::<&[u32]>);
            self.cached_prefix_tokens = None;
        }

        let mut all_tokens = Vec::with_capacity(tokens.len() + max);
        all_tokens.extend_from_slice(&tokens);
        let mut pos = 0;

        let start = Instant::now();

        // Prefill
        for &token in &tokens {
            if self.layer_prefetch {
                if let Some(ref mmap) = self.mmap {
                    crate::perf_topology::prefetch_next_layer(mmap, pos * 4096, 1 << 20);
                }
            }
            let input = Tensor::new(&[token], &self.device)?.unsqueeze(0)?;
            let _logits = self.model.forward(&input, pos)?;
            pos += 1;
        }

        // Store prefix KV after prefill if prefix caching enabled and this is a new prefix.
        if self.kv_prefix_cache && !use_prefix_kv && !prefix_tokens.is_empty() {
            self.cache
                .store_prefix_kv(&prefix_tokens, prefix_tokens.len());
            self.cached_prefix_tokens = Some(prefix_tokens);
        }

        let mut output_stream = TokenOutputStream::new(self.tokenizer.clone());
        let mut generated = 0usize;
        if self.logits_scratch.capacity() < 32000 {
            self.logits_scratch
                .reserve(32000 - self.logits_scratch.capacity());
        }
        // Train the n-gram drafter on prompt + history before drafting.
        self.train_speculative_drafter(&tokens);
        let mut spec_allow = self.speculative_enabled;
        // Tokenizer decode is batched per WS coalesce window (50ms, §6.1):
        // we still push per token but the WS layer coalesces; no String
        // growth inside the hot loop beyond the coalescer's buffer.
        while generated < max {
            if self.is_cancelled() {
                break;
            }
            if self.layer_prefetch {
                if let Some(ref mmap) = self.mmap {
                    let off = (pos * 4096) % mmap.len().max(1);
                    crate::perf_topology::prefetch_next_layer(mmap, off, 1 << 20);
                }
            }

            // Sliding window KV: if we exceed the window, shift the KV cache.
            if let Some(window) = self.kv_cache_window {
                if pos >= window {
                    // Shift KV cache: clear and re-feed last `window` tokens.
                    self.cache.reset(&mut self.model, None);
                    let replay_start = all_tokens.len().saturating_sub(window);
                    let replay_tokens = &all_tokens[replay_start..];
                    pos = 0;
                    for &token in replay_tokens {
                        if self.layer_prefetch {
                            if let Some(ref mmap) = self.mmap {
                                crate::perf_topology::prefetch_next_layer(
                                    mmap,
                                    pos * 4096,
                                    1 << 20,
                                );
                            }
                        }
                        let input = Tensor::new(&[token], &self.device)?.unsqueeze(0)?;
                        let _ = self.model.forward(&input, pos)?;
                        pos += 1;
                    }
                }
            }

            if self.layer_prefetch {
                if let Some(ref mmap) = self.mmap {
                    let off = (pos * 4096) % mmap.len().max(1);
                    crate::perf_topology::prefetch_next_layer(mmap, off, 1 << 20);
                }
            }

            // Speculative decoding (streaming twin of the batch path): same
            // Levi/greedy accept, each committed token is emitted as a stream
            // event before moving on.
            if self.speculative_enabled && spec_allow {
                let remaining = max - generated;
                let max_draft = self
                    .speculative_decoder
                    .as_ref()
                    .map(|d| d.max_draft_tokens)
                    .unwrap_or(0)
                    .min(remaining.saturating_sub(1));
                if max_draft > 0 {
                    if let Ok((committed, forwards)) = self.speculative_step(&all_tokens, max_draft)
                    {
                        if !committed.is_empty() {
                            debug_assert_eq!(forwards, committed.len());
                            debug_assert!(committed.len() <= remaining);
                            let mut hit_eos = false;
                            for &tok in &committed {
                                all_tokens.push(tok);
                                generated += 1;
                                pos += 1;
                                if let Ok(Some(text)) = output_stream.next_token(tok) {
                                    let _ = tx.blocking_send(StreamEvent::Token {
                                        token_id: tok,
                                        text,
                                        pos: pos - 1,
                                    });
                                }
                                if tok == self.eos_token_id {
                                    hit_eos = true;
                                    break;
                                }
                            }
                            if hit_eos || generated >= max {
                                break;
                            }
                            spec_allow = self
                                .speculative_decoder
                                .as_ref()
                                .map(|d| {
                                    d.total_drafted == 0
                                        || d.acceptance_rate() >= d.min_acceptance_rate
                                })
                                .unwrap_or(false);
                            continue;
                        }
                    }
                }
            }

            let last_token = *all_tokens.last().unwrap();
            let input = Tensor::new(&[last_token], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?;

            let next_token =
                self.sampler
                    .sample_with_scratch(&logits, &all_tokens, &mut self.logits_scratch)?;
            all_tokens.push(next_token);
            generated += 1;
            pos += 1;

            if let Ok(Some(text)) = output_stream.next_token(next_token) {
                let _ = tx.blocking_send(StreamEvent::Token {
                    token_id: next_token,
                    text,
                    pos: pos - 1,
                });
            }

            if next_token == self.eos_token_id {
                break;
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        let tps = if elapsed > 0.0 {
            generated as f64 / elapsed
        } else {
            0.0
        };

        let full_text = output_stream.decode_all()?;
        let _ = tx.blocking_send(StreamEvent::Done {
            total_tokens: generated,
            tokens_per_second: tps,
            full_text: full_text.clone(),
        });

        self.conversation_history.push(ChatMessage {
            role: MessageRole::User,
            content: user_message.to_string(),
        });
        self.conversation_history.push(ChatMessage {
            role: MessageRole::Assistant,
            content: full_text,
        });

        self.trim_history();

        Ok(())
    }

    fn trim_history(&mut self) {
        let max_messages = self.max_history_turns * 2;
        while self.conversation_history.len() > max_messages {
            self.conversation_history.remove(0);
            self.conversation_history.remove(0);
        }
    }

    pub fn device_type(&self) -> String {
        format!("{:?}", self.device)
    }

    /// Backing candle device (used to carry device type across a routing swap).
    pub fn device(&self) -> &candle_core::Device {
        &self.device
    }

    /// Advertised context budget from the active model configuration.
    pub fn max_seq_len(&self) -> usize {
        self.config.max_seq_len
    }

    pub fn simd_level(&self) -> &str {
        &self.simd
    }

    pub fn model_architecture(&self) -> &ModelArchitecture {
        self.config
            .architecture
            .as_ref()
            .unwrap_or(&ModelArchitecture::Llama)
    }

    pub fn set_temperature(&mut self, temp: f64) {
        self.sampler.set_temperature(temp);
    }

    pub fn reload_with_config(&mut self, new_config: &LlmConfig) -> Result<()> {
        let device = new_config.device_type.to_device();

        let file = File::open(&new_config.model_path).map_err(VortexAtomsError::Io)?;
        let mut buf_reader = BufReader::new(file);
        let content = gguf_file::Content::read(&mut buf_reader)
            .map_err(|e| VortexAtomsError::Gguf(e.to_string()))?;

        let arch = match &new_config.architecture {
            Some(a) => a.clone(),
            None => detect_architecture(&content)?,
        };

        let eos_token_id = detect_eos_token_id(&content);

        let file_for_model = File::open(&new_config.model_path).map_err(VortexAtomsError::Io)?;
        let mut buf_reader_for_model = BufReader::new(file_for_model);
        let model = load_gguf_model(
            content,
            &mut buf_reader_for_model,
            &device,
            &arch,
            new_config.use_flash_attn,
        )?;

        let tokenizer = Tokenizer::from_file(&new_config.tokenizer_path)
            .map_err(VortexAtomsError::Tokenizers)?;

        self.model = model;
        self.tokenizer = tokenizer;
        self.eos_token_id = eos_token_id;
        self.config = new_config.clone();
        self.template = ChatTemplate::default_for_arch(&arch.to_string());
        self.cache = VortexCache::new(new_config.max_seq_len);
        self.sampler = VortexSampler::new(new_config.seed, &new_config.sampling);
        self.simd = build_info::simd_summary();
        self.device = device;

        // Rebuild speculative fields (drafter tables are not model-dependent,
        // but enable/params/seed follow the live performance config).
        let perf = crate::vortex_config::VortexConfig::load().performance;
        self.speculative_enabled = perf.speculative_enabled;
        self.speculative_decoder = if perf.speculative_enabled {
            let drafter = crate::speculative::NGramDrafter::new(
                perf.ngram_order,
                32000,
                new_config.sampling.temperature.unwrap_or(0.8) as f32,
                new_config.seed,
            );
            Some(crate::speculative::SpeculativeDecoder::new_seeded(
                Box::new(drafter),
                perf.max_draft_tokens,
                perf.min_acceptance_rate,
                new_config.seed ^ 0x5EED_1A7E,
            ))
        } else {
            None
        };

        Ok(())
    }

    pub fn model_size_params(&self) -> u64 {
        self.config.model_size_params
    }

    pub fn model_quantization(&self) -> &str {
        &self.config.model_quantization
    }

    pub fn swap_model(&mut self, new_config: &LlmConfig) -> Result<()> {
        self.reload_with_config(new_config)?;
        self.conversation_history.clear();
        self.cache.reset(&mut self.model, None::<&[u32]>);
        println!(
            "[Vortex] Swapped to model: {} (size={}, quant={})",
            new_config.model_path.display(),
            new_config.model_size_params,
            new_config.model_quantization,
        );
        Ok(())
    }

    pub fn batch_generate(
        &mut self,
        prompts: &[&str],
        max_tokens: Option<usize>,
    ) -> Result<Vec<String>> {
        let mut results = Vec::with_capacity(prompts.len());

        for prompt in prompts {
            self.clear_history();
            let result = self.generate(prompt, max_tokens)?;
            results.push(result);
        }

        Ok(results)
    }

    pub fn count_tokens(&self, text: &str) -> Result<usize> {
        self.tokenize(text).map(|t| t.len())
    }
}
