use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use crate::state::now_epoch_ms;

/// Zero-cost token-cache contract.
///
/// This trait is intended for static dispatch. Callers are generic over
/// `TokenCache<K>`, so the compiler can monomorphize the cache access path
/// without virtual calls or heap-allocated trait objects.
pub trait TokenCache<K>
where
    K: Eq + Hash,
{
    type Block: Clone;

    fn get_cached(&mut self, key: &K) -> Option<Self::Block>;
    fn insert_cached(&mut self, key: K, block: Self::Block);
    fn remove_cached(&mut self, key: &K) -> Option<Self::Block>;
}

/// Immutable token block for frequently requested cognitive data.
///
/// The underlying token array is shared through `Arc<[u32]>`, so cache hits only
/// clone a pointer-sized handle and never perform a disk read or duplicate token
/// memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CognitiveTokenBlock {
    pub fragment_id: String,
    pub tokens: Arc<[u32]>,
    pub source_bytes: usize,
    pub created_epoch_ms: u128,
    pub last_access_epoch_ms: u128,
    pub hits: u64,
}

impl CognitiveTokenBlock {
    pub fn from_bytes(fragment_id: impl Into<String>, bytes: &[u8]) -> Self {
        let now = now_epoch_ms();
        Self {
            fragment_id: fragment_id.into(),
            tokens: tokenize_cognitive_bytes(bytes),
            source_bytes: bytes.len(),
            created_epoch_ms: now,
            last_access_epoch_ms: now,
            hits: 0,
        }
    }

    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

/// Hot in-memory token cache used as a read-through layer by Kernel_02.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotTokenCache {
    blocks: HashMap<String, CognitiveTokenBlock>,
    capacity_tokens: usize,
    total_tokens: usize,
}

impl HotTokenCache {
    pub fn new(capacity_tokens: usize) -> Self {
        Self {
            blocks: HashMap::new(),
            capacity_tokens: capacity_tokens.max(1),
            total_tokens: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn total_tokens(&self) -> usize {
        self.total_tokens
    }

    pub fn contains(&self, key: &str) -> bool {
        self.blocks.contains_key(key)
    }

    pub fn evict_inactive_older_than(&mut self, older_than_ms: u128) -> usize {
        let now = now_epoch_ms();
        let keys_to_remove = self
            .blocks
            .iter()
            .filter_map(|(key, block)| {
                let age_ms = now.saturating_sub(block.last_access_epoch_ms);
                (age_ms >= older_than_ms).then(|| key.clone())
            })
            .collect::<Vec<_>>();

        let removed = keys_to_remove
            .iter()
            .filter_map(|key| self.remove_cached(key))
            .collect::<Vec<_>>();
        let removed_count = removed.len();

        // Immediate release point for evicted hot-token arrays.
        std::mem::drop(removed);

        removed_count
    }

    fn enforce_capacity(&mut self) {
        while self.total_tokens > self.capacity_tokens && !self.blocks.is_empty() {
            // Tie-break on key so equal (hits, last_access) always evicts
            // the same victim regardless of HashMap iteration order.
            let coldest_key = self
                .blocks
                .iter()
                .min_by_key(|(key, block)| (block.hits, block.last_access_epoch_ms, key.as_str()))
                .map(|(key, _)| key.clone());

            if let Some(key) = coldest_key {
                let _ = self.remove_cached(&key);
            } else {
                break;
            }
        }
    }
}

impl Default for HotTokenCache {
    fn default() -> Self {
        Self::new(1_000_000)
    }
}

impl TokenCache<String> for HotTokenCache {
    type Block = CognitiveTokenBlock;

    fn get_cached(&mut self, key: &String) -> Option<Self::Block> {
        self.blocks.get_mut(key).map(|block| {
            block.hits = block.hits.saturating_add(1);
            block.last_access_epoch_ms = now_epoch_ms();
            block.clone()
        })
    }

    fn insert_cached(&mut self, key: String, block: Self::Block) {
        if let Some(previous) = self.blocks.insert(key, block.clone()) {
            self.total_tokens = self.total_tokens.saturating_sub(previous.token_count());
        }
        self.total_tokens = self.total_tokens.saturating_add(block.token_count());
        self.enforce_capacity();
    }

    fn remove_cached(&mut self, key: &String) -> Option<Self::Block> {
        let removed = self.blocks.remove(key);
        if let Some(block) = &removed {
            self.total_tokens = self.total_tokens.saturating_sub(block.token_count());
        }
        removed
    }
}

/// Deterministic lightweight tokenization for opaque cognitive fragments.
///
/// `.tcz` and `.bin` files are treated as compressed/opaque byte sources here.
/// The runtime keeps the original compressed bytes mmap/disk-friendly and caches
/// derived token words in RAM only for frequently requested fragments.
pub fn tokenize_cognitive_bytes(bytes: &[u8]) -> Arc<[u32]> {
    if bytes.is_empty() {
        return Arc::from(Vec::<u32>::new().into_boxed_slice());
    }

    let mut tokens = Vec::with_capacity(bytes.len().div_ceil(4));
    for chunk in bytes.chunks(4) {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        tokens.push(u32::from_le_bytes(word));
    }

    Arc::from(tokens.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_tie_evicts_lexicographically_smallest_key() {
        // Each 8-byte block → 2 tokens. Capacity 4 fits two blocks; the
        // third insert overflows. Equal hits + equal last_access → id wins.
        let mut cache = HotTokenCache::new(4);
        let fixed_ts = 1_700_000_000_000u128;
        let mk = |key: &str| {
            let mut block = CognitiveTokenBlock::from_bytes(key, &[1, 2, 3, 4, 5, 6, 7, 8]);
            block.last_access_epoch_ms = fixed_ts;
            (key.to_string(), block)
        };

        let (ka, ba) = mk("aa");
        let (km, bm) = mk("mm");
        let (kz, bz) = mk("zz");

        cache.insert_cached(ka, ba);
        cache.insert_cached(km, bm);
        cache.insert_cached(kz, bz);

        assert!(cache.get_cached(&"aa".to_string()).is_none());
        assert!(cache.get_cached(&"mm".to_string()).is_some());
        assert!(cache.get_cached(&"zz".to_string()).is_some());
    }
}
