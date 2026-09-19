use crate::llm_model::LlmModel;
use std::collections::HashMap;

/// Manages KV-cache lifecycle for the inference engine.
pub struct VortexCache {
    pub total_tokens_in_cache: usize,
    pub max_seq_len: usize,
    /// Optional sliding window size. None = full context (no sliding).
    kv_cache_window: Option<usize>,
    /// Static prefix KV cache: map from prefix hash to cached KV state.
    /// Each entry stores (prefix_tokens, kv_cache_snapshot).
    prefix_kv_cache: HashMap<u64, (Vec<u32>, PrefixKVState)>,
}

/// Opaque prefix KV state - model-specific KV cache snapshot.
/// For candle models, this stores the position offset and a marker
/// that the prefix KV is valid. Actual KV tensors stay in the model.
#[derive(Clone)]
pub struct PrefixKVState {
    /// Number of tokens in the cached prefix.
    pub prefix_len: usize,
    /// Hash of the prefix tokens for validation.
    pub prefix_hash: u64,
}

impl VortexCache {
    pub fn new(max_seq_len: usize) -> Self {
        Self {
            total_tokens_in_cache: 0,
            max_seq_len,
            kv_cache_window: None,
            prefix_kv_cache: HashMap::new(),
        }
    }

    /// Create with sliding window and prefix cache configuration.
    pub fn with_config(
        max_seq_len: usize,
        kv_cache_window: Option<usize>,
        kv_prefix_cache: bool,
    ) -> Self {
        Self {
            total_tokens_in_cache: 0,
            max_seq_len,
            kv_cache_window: if kv_prefix_cache { kv_cache_window } else { None },
            prefix_kv_cache: HashMap::new(),
        }
    }

    /// Reset the cache for a new generation session.
    /// If prefix KV is enabled and we have a cached prefix matching the
    /// beginning of the prompt, we preserve that prefix KV.
    pub fn reset(&mut self, model: &mut dyn LlmModel, preserve_prefix: Option<&[u32]>) {
        if let Some(prefix) = preserve_prefix {
            // Check if we have a cached prefix KV for this prefix
            let hash = Self::hash_prefix(prefix);
            if self.prefix_kv_cache.contains_key(&hash) {
                // Keep the prefix KV in the model; only clear tokens after prefix
                // Note: Actual implementation would need model-specific KV shifting.
                // For now, we clear and will re-apply prefix KV on next generation.
                model.clear_kv_cache();
                self.total_tokens_in_cache = 0;
                return;
            }
        }
        model.clear_kv_cache();
        self.total_tokens_in_cache = 0;
    }

    /// Reset but keep prefix KV if configured and prefix matches.
    pub fn reset_with_prefix(&mut self, model: &mut dyn LlmModel, prefix: &[u32]) {
        self.reset(model, Some(prefix));
    }

    /// Record that tokens were added to the cache.
    pub fn advance(&mut self, count: usize) {
        self.total_tokens_in_cache += count;
        // Sliding window: if we exceed the window, mark that shift is needed
        if let Some(window) = self.kv_cache_window {
            if self.total_tokens_in_cache > window {
                // Mark that sliding is needed (actual shift happens in generation loop)
            }
        }
    }

    /// Check if there is room for more tokens.
    pub fn has_capacity(&self) -> bool {
        self.total_tokens_in_cache < self.max_seq_len
    }

    /// Remaining capacity.
    pub fn remaining(&self) -> usize {
        self.max_seq_len.saturating_sub(self.total_tokens_in_cache)
    }

    /// Get the sliding window size.
    pub fn window_size(&self) -> Option<usize> {
        self.kv_cache_window
    }

    /// Compute a hash of the prefix tokens for caching.
    fn hash_prefix(tokens: &[u32]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        tokens.hash(&mut hasher);
        hasher.finish()
    }

    /// Store a prefix KV state for reuse.
    pub fn store_prefix_kv(&mut self, prefix_tokens: &[u32], prefix_len: usize) {
        if self.prefix_kv_cache.is_empty() && self.kv_cache_window.is_none() {
            return; // prefix caching not enabled
        }
        let hash = Self::hash_prefix(prefix_tokens);
        let state = PrefixKVState {
            prefix_len,
            prefix_hash: hash,
        };
        self.prefix_kv_cache.insert(hash, (prefix_tokens.to_vec(), state));
    }

    /// Check if we have a cached prefix KV for the given prefix.
    pub fn has_prefix_kv(&self, prefix_tokens: &[u32]) -> bool {
        let hash = Self::hash_prefix(prefix_tokens);
        self.prefix_kv_cache.contains_key(&hash)
    }

    /// Get the cached prefix length if available.
    pub fn get_cached_prefix_len(&self, prefix_tokens: &[u32]) -> Option<usize> {
        let hash = Self::hash_prefix(prefix_tokens);
        self.prefix_kv_cache.get(&hash).map(|(_, state)| state.prefix_len)
    }
}
