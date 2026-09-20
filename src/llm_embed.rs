use std::sync::Mutex;

use crate::llm_neural_embed::NeuralEmbedder;
use crate::Result;

/// Fast hash-based embedder (FNV-1a over whitespace tokens, L2-normalized).
///
/// This is a **lexical hash fallback for offline/minimal operation — NOT a
/// neural semantic embedding**. Same tokens hash to the same buckets, so
/// exact/near-duplicate text scores highly, but paraphrases with different
/// wording do not. Use the `neural-embed` feature (`OrtEmbedder`) when true
/// semantic similarity is required.
pub struct FastHashEmbedder {
    dimensions: usize,
}

/// Backwards-compatible alias. Prefer `FastHashEmbedder` in new code.
pub type EmbeddingModel = FastHashEmbedder;

impl FastHashEmbedder {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut vector = vec![0.0f32; self.dimensions];

        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(vector);
        }

        for (i, token) in tokens.iter().enumerate() {
            let mut hash = 2_166_136_261u32;
            for byte in token.bytes() {
                hash ^= u32::from(byte);
                hash = hash.wrapping_mul(16_777_619);
            }

            let idx = (hash as usize) % self.dimensions;
            let sign = if (hash >> 31) & 1 == 0 { 1.0 } else { -1.0 };
            let magnitude = 1.0 / (1.0 + (i as f32));
            vector[idx] += sign * magnitude;
        }

        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in vector.iter_mut() {
                *v /= norm;
            }
        }

        Ok(vector)
    }

    pub fn batch_embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

impl NeuralEmbedder for FastHashEmbedder {
    fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        FastHashEmbedder::embed(self, text)
    }

    fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-10 {
        0.0
    } else {
        dot / denom
    }
}

/// Future seam for approximate-nearest-neighbor indexes.
///
/// The only implementation today is the linear scan inside [`VectorStore`]
/// — retrieval complexity O(N·D) per query (N vectors, D dimensions).
/// A future `AnnIndex` can implement this trait without touching callers:
/// embed the query once, then call `search_by_vector`.
///
/// Measured on i5-2430M (D=64, top_k=5, `bench_linear_search_scaling`):
/// N=200 → 460 µs/query; N=1000 → 2314 µs/query; N=5000 → 11526 µs/query
/// (~2.3 µs/vector — textbook linear scaling, no index).
pub trait VectorIndex {
    fn index_len(&self) -> usize;

    fn search_by_vector(&self, query: &[f32], top_k: usize) -> Vec<SearchResult>;
}

pub struct VectorStore {
    embeddings: Vec<(String, Vec<f32>, String)>,
    model: Mutex<Box<dyn NeuralEmbedder>>,
}

impl VectorStore {
    pub fn new(dimensions: usize) -> Self {
        Self {
            embeddings: Vec::new(),
            model: Mutex::new(
                Box::new(FastHashEmbedder::new(dimensions)) as Box<dyn NeuralEmbedder>
            ),
        }
    }

    pub fn insert(&mut self, id: impl Into<String>, text: impl Into<String>) -> Result<()> {
        let id = id.into();
        let text = text.into();
        let embedding = self.model.lock().unwrap().embed(&text)?;
        self.embeddings.push((id, embedding, text));
        Ok(())
    }

    pub fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let query_embedding = self.model.lock().unwrap().embed(query)?;
        Ok(self.search_by_vector(&query_embedding, top_k))
    }

    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.embeddings.len();
        self.embeddings.retain(|(eid, _, _)| eid != id);
        self.embeddings.len() < before
    }

    pub fn clear(&mut self) {
        self.embeddings.clear();
    }

    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        self.model.lock().unwrap().embed(text)
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.model.lock().unwrap().embed_batch(texts)
    }

    pub fn dimensions(&self) -> usize {
        self.model.lock().unwrap().dimensions()
    }

    pub fn set_embedder(&mut self, embedder: Box<dyn NeuralEmbedder>) {
        *self.model.lock().unwrap() = embedder;
    }
}

impl VectorIndex for VectorStore {
    fn index_len(&self) -> usize {
        self.embeddings.len()
    }

    /// Linear scan over all stored vectors: O(N·D).
    /// Documented complexity — see [`VectorIndex`].
    fn search_by_vector(&self, query: &[f32], top_k: usize) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .embeddings
            .iter()
            .map(|(id, embedding, text)| {
                let score = cosine_similarity(query, embedding);
                SearchResult {
                    id: id.clone(),
                    text: text.clone(),
                    score,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);

        results
    }
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub id: String,
    pub text: String,
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_store(n: usize) -> VectorStore {
        let mut store = VectorStore::new(64);
        for i in 0..n {
            store
                .insert(
                    format!("doc_{i}"),
                    format!("document number {i} about rust systems"),
                )
                .unwrap();
        }
        store
    }

    #[test]
    fn search_exact_match_ranks_first() {
        let store = seeded_store(50);
        let results = store
            .search("document number 7 about rust systems", 5)
            .unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc_7");
        assert!(results[0].score > 0.99);
    }

    #[test]
    fn search_respects_top_k_and_order() {
        let store = seeded_store(50);
        let results = store.search("rust systems", 5).unwrap();
        assert_eq!(results.len(), 5);
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn search_by_vector_matches_search() {
        let store = seeded_store(20);
        let query = store.embed_text("document number 3").unwrap();
        let via_trait = store.search_by_vector(&query, 3);
        let via_search = store.search("document number 3", 3).unwrap();
        assert_eq!(via_trait.len(), via_search.len());
        assert_eq!(via_trait[0].id, via_search[0].id);
    }

    #[test]
    fn search_empty_store_returns_empty() {
        let store = VectorStore::new(64);
        let results = store.search("anything", 5).unwrap();
        assert!(results.is_empty());
        assert_eq!(store.index_len(), 0);
    }

    #[test]
    fn hash_embedder_is_deterministic() {
        let model = FastHashEmbedder::new(64);
        let a = model.embed("hello world").unwrap();
        let b = model.embed("hello world").unwrap();
        assert_eq!(a, b);
        // Backwards-compat alias resolves to the same type.
        let via_alias = EmbeddingModel::new(64);
        assert_eq!(via_alias.dimensions(), 64);
    }

    /// Stage 14 micro-benchmark: hash-embed throughput (D=64).
    /// Ignored in normal runs; execute explicitly and record numbers.
    #[test]
    #[ignore]
    fn bench_hash_embed_throughput() {
        let model = FastHashEmbedder::new(64);
        let text = "the quick brown fox jumps over arabic الثعلب السريع ".repeat(20);
        let iters = 2000;
        let start = std::time::Instant::now();
        for _ in 0..iters {
            let _ = model.embed(&text).unwrap();
        }
        let per_us = start.elapsed().as_micros() / iters as u128;
        println!("hash_embed D=64 ~1KB text: {per_us} µs/embed");
    }

    /// Scaling benchmark: linear scan must grow ~linearly with N.
    /// Ignored in normal runs; execute explicitly and record numbers.
    #[test]
    #[ignore]
    fn bench_linear_search_scaling() {
        for n in [200usize, 1000, 5000] {
            let store = seeded_store(n);
            let query = store.embed_text("rust systems benchmark").unwrap();
            let start = std::time::Instant::now();
            let iters = 20;
            for _ in 0..iters {
                let _ = store.search_by_vector(&query, 5);
            }
            let per_query_us = start.elapsed().as_micros() / iters as u128;
            println!("linear_search N={n}: {per_query_us} µs/query (D=64)");
        }
    }
}
