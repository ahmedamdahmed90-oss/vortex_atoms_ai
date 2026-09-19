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
    /// Layer double-buffer prefetch toggle (1.4, config `layer_prefetch`).
    layer_prefetch: bool,
    /// Sliding window KV cache size (config `kv_cache_window`).
    kv_cache_window: Option<usize>,
    /// Static prefix KV cache enabled (config `kv_prefix_cache`).
    kv_prefix_cache: bool,
    /// Cached prefix tokens for prefix KV reuse.
    cached_prefix_tokens: Option<Vec<u32>>,
    /// Speculative decoding: n-gram drafter.
    speculative_decoder: Option<crate::speculative::SpeculativeDecoder>,
    /// Speculative decoding enabled (config `speculative_enabled`).
    speculative_enabled: bool,
    /// N-gram drafter trained on conversation history.
    ngram_drafter: Option<crate::speculative::NGramDrafter>,
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

        // Initialize speculative decoding if enabled
        let (speculative_decoder, ngram_drafter) = if speculative_enabled {
            let drafter = crate::speculative::NGramDrafter::new(
                ngram_order,
                32000, // vocab size
                config.sampling.temperature.unwrap_or(0.8) as f32,
                config.seed,
            );
            let decoder = crate::speculative::SpeculativeDecoder::new(
                Box::new(drafter.clone()),
                max_draft_tokens,
                min_acceptance_rate,
            );
            (Some(decoder), Some(drafter))
        } else {
            (None, None)
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
            layer_prefetch,
            kv_cache_window,
            kv_prefix_cache,
            cached_prefix_tokens: None,
            speculative_decoder,
            speculative_enabled,
            ngram_drafter,
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
        let system_text = format_chat_history(&self.template, std::slice::from_ref(system_msg), None);
        self.tokenize(&system_text).unwrap_or_default()
    }

    /// Get the cached prefix tokens if available.
    pub fn get_cached_prefix(&self) -> Option<&[u32]> {
        self.cached_prefix_tokens.as_deref()
    }

    /// Verify a batch of drafted tokens by running the model on the full sequence.
    /// Returns logits for each position in the sequence.
    fn verify_drafted_tokens(&mut self, sequence: &[u32]) -> Result<Vec<Vec<f32>>> {
        let mut logits_seq = Vec::with_capacity(sequence.len());
        for (pos, &token) in sequence.iter().enumerate() {
            if self.layer_prefetch {
                if let Some(ref mmap) = self.mmap {
                    crate::perf_topology::prefetch_next_layer(mmap, pos * 4096, 1 << 20);
                }
            }
            let input = Tensor::new(&[token], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?;
            let logits_vec = logits.to_vec1::<f32>()?;
            logits_seq.push(logits_vec);
        }
        Ok(logits_seq)
    }

    /// Train the n-gram drafter on the given tokens.
    #[allow(dead_code)]
    fn train_ngram_drafter(&mut self, tokens: &[u32]) {
        if let Some(drafter) = &mut self.ngram_drafter {
            drafter.train(tokens);
        }
    }

    /// Collect tokens from conversation history and system prompt for drafter training.
    #[allow(dead_code)]
    fn collect_drafter_training_tokens(&self) -> Vec<u32> {
        let mut all_tokens = Vec::new();
        for msg in &self.conversation_history {
            if let Ok(tokens) = self.tokenize(&msg.content) {
                all_tokens.extend(tokens);
            }
        }
        if let Ok(tokens) = self.tokenize(&self.config.system_prompt) {
            all_tokens.extend(tokens);
        }
        all_tokens
    }

    /// Speculative generation step: draft tokens and verify them.
    /// Returns (accepted_tokens, should_continue_speculative).
    fn speculative_step(
        &mut self,
        context: &[u32],
        max_draft: usize,
    ) -> Result<(Vec<u32>, bool)> {
        if let Some(decoder) = &mut self.speculative_decoder {
            // Get logits for the full sequence (context + drafted)
            let drafted = decoder.drafter.draft(context, max_draft);
            if drafted.is_empty() {
                return Ok((Vec::new(), false));
            }

            let mut full_seq = context.to_vec();
            full_seq.extend_from_slice(&drafted);

            let logits_seq = self.verify_drafted_tokens(&full_seq)?;

            let mut accepted = Vec::new();
            for (i, &drafted_tok) in drafted.iter().enumerate() {
                let pos = context.len() + i;
                if pos < logits_seq.len() {
                    let logits = &logits_seq[pos];
                    let best_tok = crate::llm_sampling::argmax_index(logits) as u32;
                    if best_tok == drafted_tok {
                        accepted.push(drafted_tok);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            let accepted_count = accepted.len();
            let acceptance_rate = if max_draft > 0 {
                accepted_count as f32 / max_draft as f32
            } else {
                0.0
            };

            if !accepted.is_empty() {
                if let Some(decoder) = &mut self.speculative_decoder {
                    decoder.drafter.update(&accepted);
                }
            }

            let continue_speculative = acceptance_rate >= 0.5;
            Ok((accepted, continue_speculative))
        } else {
            Ok((Vec::new(), false))
        }
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
            self.cache.reset_with_prefix(&mut self.model, &prefix_tokens);
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
            self.cache.store_prefix_kv(&prefix_tokens, prefix_tokens.len());
            self.cached_prefix_tokens = Some(prefix_tokens);
        }

        let start = Instant::now();
        let mut generated = 0usize;
        // Ensure logits scratch has vocab capacity once (reuse, no per-token alloc).
        if self.logits_scratch.capacity() < 32000 {
            self.logits_scratch.reserve(32000 - self.logits_scratch.capacity());
        }

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
                                crate::perf_topology::prefetch_next_layer(mmap, pos * 4096, 1 << 20);
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

// Speculative decoding: try to draft and verify multiple tokens at once.
            if self.speculative_enabled {
                if let Some(decoder) = &mut self.speculative_decoder {
                    let max_draft = decoder.max_draft_tokens;
                    if let Ok((accepted, _continue_spec)) = self.speculative_step(&all_tokens, max_draft) {
                        for &tok in &accepted {
                            all_tokens.push(tok);
                            generated += 1;
                            pos += 1;
                            if tok == self.eos_token_id {
                                break;
                            }
                        }
                        if generated >= max {
                            break;
                        }
                        if !accepted.is_empty() {
                            // Speculative step succeeded, continue to next iteration
                            continue;
                        }
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
            let next_token = self.sampler.sample_with_scratch(&logits, &all_tokens, &mut self.logits_scratch)?;
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
            self.cache.reset_with_prefix(&mut self.model, &prefix_tokens);
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
            self.cache.store_prefix_kv(&prefix_tokens, prefix_tokens.len());
            self.cached_prefix_tokens = Some(prefix_tokens);
        }

        let mut output_stream = TokenOutputStream::new(self.tokenizer.clone());
        let mut generated = 0usize;
        if self.logits_scratch.capacity() < 32000 {
            self.logits_scratch.reserve(32000 - self.logits_scratch.capacity());
        }
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
                                crate::perf_topology::prefetch_next_layer(mmap, pos * 4096, 1 << 20);
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

            let last_token = *all_tokens.last().unwrap();
            let input = Tensor::new(&[last_token], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?;

            let next_token = self.sampler.sample_with_scratch(&logits, &all_tokens, &mut self.logits_scratch)?;
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
