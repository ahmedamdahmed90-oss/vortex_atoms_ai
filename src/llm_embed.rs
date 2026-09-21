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

    /// Build a trained IVF index over this store's current entries.
    /// One-line opt-in to approximate retrieval; the store itself keeps
    /// serving exact linear results unchanged.
    pub fn trained_ivf(&self, nlist: usize, nprobe: usize) -> Result<IvfIndex> {
        let mut ivf = IvfIndex::new(self.dimensions());
        ivf.set_nlist(nlist);
        ivf.set_nprobe(nprobe);
        for (id, embedding, text) in &self.embeddings {
            ivf.insert(id.clone(), embedding.clone(), text.clone())?;
        }
        ivf.train()?;
        Ok(ivf)
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

/// IVF (inverted-file) approximate index behind [`VectorIndex`].
///
/// Coarse spherical k-means over `nlist` centroids; a query probes the
/// `nprobe` nearest lists and re-ranks gathered candidates with exact
/// cosine. Per-query cost is O(nlist·D + nprobe·(N/nlist)·D) instead of
/// linear O(N·D).
///
/// Recall depends on clusterability: excellent on clustered embeddings
/// (e.g. neural), poor on near-orthogonal hash vectors
/// ([`FastHashEmbedder`]) where every centroid is ~equidistant — see the
/// recall harness in tests. The default backend stays linear; IVF is
/// opt-in (`VectorStore::trained_ivf`) for N where it wins (measured
/// crossover ≈ N 5k–10k at D=64, docs/BENCHMARKS.md).
///
/// Invariant: any mutation (insert/remove/clear/set_nlist) invalidates
/// training, and an untrained index answers by exact linear scan — so a
/// trained-or-not index is never wrong, only slower until `train()`.
pub struct IvfIndex {
    dim: usize,
    nlist: usize,
    nprobe: usize,
    centroids: Vec<Vec<f32>>,
    lists: Vec<Vec<usize>>,
    entries: Vec<(String, Vec<f32>, String)>,
}

/// Lloyd iterations for k-means training (fixed count → deterministic).
const IVF_TRAIN_ITERS: usize = 10;

fn l2_normalize_into(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < 1e-10 {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

impl IvfIndex {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            nlist: 64,
            nprobe: 8,
            centroids: Vec::new(),
            lists: Vec::new(),
            entries: Vec::new(),
        }
    }

    pub fn set_nlist(&mut self, nlist: usize) {
        self.nlist = nlist.max(1);
        self.invalidate();
    }

    pub fn set_nprobe(&mut self, nprobe: usize) {
        // Query-time only: never invalidates training.
        self.nprobe = nprobe.max(1);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn is_trained(&self) -> bool {
        !self.centroids.is_empty()
    }

    fn invalidate(&mut self) {
        self.centroids.clear();
        self.lists.clear();
    }

    fn check_dim(&self, vector: &[f32]) -> Result<()> {
        if vector.len() != self.dim {
            return Err(crate::error::VortexAtomsError::InvalidTensorSpec(format!(
                "IvfIndex dim {} but vector has {}",
                self.dim,
                vector.len()
            )));
        }
        Ok(())
    }

    pub fn insert(
        &mut self,
        id: impl Into<String>,
        vector: Vec<f32>,
        text: impl Into<String>,
    ) -> Result<()> {
        self.check_dim(&vector)?;
        self.entries.push((id.into(), vector, text.into()));
        self.invalidate();
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|(eid, _, _)| eid != id);
        let removed = self.entries.len() < before;
        if removed {
            self.invalidate();
        }
        removed
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.invalidate();
    }

    /// Spherical k-means: deterministic farthest-point seeding (no RNG),
    /// fixed Lloyd iterations, centroids renormalized each round for the
    /// cosine metric. Empty clusters keep their previous centroid.
    /// No-op on an empty index.
    pub fn train(&mut self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        for (.., v, _) in &self.entries {
            self.check_dim(v)?;
        }
        let k = self.nlist.min(self.entries.len());
        let d = self.dim;

        // Farthest-point seeding from entries[0].
        let mut centroids: Vec<Vec<f32>> = vec![l2_normalize_into(&self.entries[0].1)];
        while centroids.len() < k {
            let mut best_idx = 0usize;
            let mut best_dist = f32::NEG_INFINITY;
            for (i, (.., v, _)) in self.entries.iter().enumerate() {
                let nearest = centroids
                    .iter()
                    .map(|c| cosine_similarity(v, c))
                    .fold(f32::NEG_INFINITY, f32::max);
                let dist = 1.0 - nearest;
                if dist > best_dist {
                    best_dist = dist;
                    best_idx = i;
                }
            }
            centroids.push(l2_normalize_into(&self.entries[best_idx].1));
        }

        let mut assign = vec![0usize; self.entries.len()];
        for _ in 0..IVF_TRAIN_ITERS {
            // Assign.
            for (i, (.., v, _)) in self.entries.iter().enumerate() {
                let mut best = 0usize;
                let mut best_score = f32::NEG_INFINITY;
                for (c, centroid) in centroids.iter().enumerate() {
                    let s = cosine_similarity(v, centroid);
                    if s > best_score {
                        best_score = s;
                        best = c;
                    }
                }
                assign[i] = best;
            }
            // Recompute + renormalize; keep old centroid for empty clusters.
            let mut sums = vec![vec![0.0f32; d]; k];
            let mut counts = vec![0usize; k];
            for (i, (.., v, _)) in self.entries.iter().enumerate() {
                let c = assign[i];
                for (j, x) in v.iter().enumerate() {
                    sums[c][j] += *x;
                }
                counts[c] += 1;
            }
            for c in 0..k {
                if counts[c] > 0 {
                    centroids[c] = l2_normalize_into(&sums[c]);
                }
            }
        }

        // Build inverted lists.
        let mut lists: Vec<Vec<usize>> = vec![Vec::new(); k];
        for (i, (.., v, _)) in self.entries.iter().enumerate() {
            let mut best = 0usize;
            let mut best_score = f32::NEG_INFINITY;
            for (c, centroid) in centroids.iter().enumerate() {
                let s = cosine_similarity(v, centroid);
                if s > best_score {
                    best_score = s;
                    best = c;
                }
            }
            lists[best].push(i);
        }

        self.centroids = centroids;
        self.lists = lists;
        Ok(())
    }

    fn nearest_centroids(&self, query: &[f32], n: usize) -> Vec<usize> {
        let mut scored: Vec<(usize, f32)> = self
            .centroids
            .iter()
            .enumerate()
            .map(|(c, centroid)| (c, cosine_similarity(query, centroid)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(n.min(self.centroids.len()))
            .map(|(c, _)| c)
            .collect()
    }

    fn linear_results(&self, query: &[f32], top_k: usize) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .map(|(id, embedding, text)| SearchResult {
                id: id.clone(),
                text: text.clone(),
                score: cosine_similarity(query, embedding),
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

impl VectorIndex for IvfIndex {
    fn index_len(&self) -> usize {
        self.entries.len()
    }

    /// Untrained → exact linear scan. Trained → probe `nprobe` lists,
    /// exact cosine re-rank of gathered candidates.
    fn search_by_vector(&self, query: &[f32], top_k: usize) -> Vec<SearchResult> {
        if !self.is_trained() {
            return self.linear_results(query, top_k);
        }
        let mut seen = vec![false; self.entries.len()];
        let mut candidates: Vec<usize> = Vec::new();
        for c in self.nearest_centroids(query, self.nprobe) {
            for &i in &self.lists[c] {
                if !seen[i] {
                    seen[i] = true;
                    candidates.push(i);
                }
            }
        }
        let mut results: Vec<SearchResult> = candidates
            .into_iter()
            .map(|i| {
                let (id, embedding, text) = &self.entries[i];
                SearchResult {
                    id: id.clone(),
                    text: text.clone(),
                    score: cosine_similarity(query, embedding),
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

    // ============================================================
    // IVF INDEX TESTS
    // ============================================================

    fn ivf_entries(n: usize, dim: usize) -> Vec<(String, Vec<f32>, String)> {
        // Deterministic pseudo-random unit vectors (xorshift, fixed seed).
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut next_f32 = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // NOTE: divisor matches the >> 11 shift: (u64::MAX >> 11) == 2^53.
            // Dividing by u64::MAX instead collapses output to [0, 2^-11) —
            // every vector then points the same way and all cosines read 1.0.
            ((state >> 11) as f32) / ((u64::MAX >> 11) as f32)
        };
        (0..n)
            .map(|i| {
                let mut v: Vec<f32> = (0..dim).map(|_| next_f32() * 2.0 - 1.0).collect();
                let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                for x in v.iter_mut() {
                    *x /= norm;
                }
                (format!("v_{i}"), v, format!("text {i}"))
            })
            .collect()
    }

    fn trained_ivf(n: usize) -> IvfIndex {
        let mut ivf = IvfIndex::new(64);
        for (id, v, t) in ivf_entries(n, 64) {
            ivf.insert(id, v, t).unwrap();
        }
        ivf.train().unwrap();
        assert!(ivf.is_trained());
        ivf
    }

    #[test]
    fn ivf_untrained_matches_exact_linear() {
        let mut ivf = IvfIndex::new(64);
        for (id, v, t) in ivf_entries(50, 64) {
            ivf.insert(id, v, t).unwrap();
        }
        assert!(!ivf.is_trained());
        let query = ivf_entries(1, 64).into_iter().next().unwrap().1;
        let got: Vec<String> = ivf
            .search_by_vector(&query, 5)
            .into_iter()
            .map(|r| r.id)
            .collect();
        // Brute-force ground truth over the same entries.
        let mut expected: Vec<(String, f32)> = ivf
            .entries
            .iter()
            .map(|(id, v, _)| (id.clone(), cosine_similarity(&query, v)))
            .collect();
        expected.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let expected_ids: Vec<String> = expected.into_iter().take(5).map(|(id, _)| id).collect();
        assert_eq!(got, expected_ids);
    }

    #[test]
    fn ivf_rejects_dim_mismatch() {
        let mut ivf = IvfIndex::new(64);
        assert!(ivf.insert("a", vec![0.0; 32], "short").is_err());
        assert!(ivf.is_empty());
    }

    #[test]
    fn ivf_mutation_invalidates_training() {
        let mut ivf = trained_ivf(200);
        ivf.insert("new".to_string(), vec![0.0; 64], "new".to_string())
            .unwrap();
        assert!(!ivf.is_trained());
        ivf.train().unwrap();
        assert!(ivf.is_trained());
        assert!(ivf.remove("new"));
        assert!(!ivf.is_trained());
        ivf.train().unwrap();
        ivf.set_nlist(16);
        assert!(!ivf.is_trained());
        ivf.train().unwrap();
        // nprobe is query-time only: never invalidates.
        ivf.set_nprobe(4);
        assert!(ivf.is_trained());
    }

    #[test]
    fn ivf_empty_and_overflow_edges() {
        let ivf = IvfIndex::new(64);
        assert!(ivf.search_by_vector(&vec![1.0; 64], 5).is_empty());
        assert_eq!(ivf.index_len(), 0);
        // nprobe larger than nlist clamps instead of panicking.
        let mut ivf = trained_ivf(100);
        ivf.set_nprobe(10_000);
        let query = vec![1.0 / 8.0; 64];
        let got = ivf.search_by_vector(&query, 200);
        assert_eq!(got.len(), 100);
        assert!(!ivf.remove("missing"));
        ivf.clear();
        assert!(ivf.is_empty());
        assert!(!ivf.is_trained());
    }

    #[test]
    fn ivf_trained_search_returns_ranked_top_k() {
        let ivf = trained_ivf(300);
        let query = ivf_entries(1, 64).into_iter().next().unwrap().1;
        let got = ivf.search_by_vector(&query, 5);
        assert_eq!(got.len(), 5);
        for w in got.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn trained_ivf_bridge_builds_trained_index() {
        let store = seeded_store(120);
        let ivf = store.trained_ivf(16, 4).unwrap();
        assert!(ivf.is_trained());
        assert_eq!(ivf.index_len(), 120);
        let query = store.embed_text("rust systems").unwrap();
        assert_eq!(ivf.search_by_vector(&query, 5).len(), 5);
    }

    // ---------- recall harness: clustered synthetic data ----------

    /// (id, vector, text) entries shared by the harness builders.
    type HarnessEntries = Vec<(String, Vec<f32>, String)>;

    struct XorShift(u64);

    impl XorShift {
        fn next_f32(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            // Divisor matches the >> 11 shift (see note in ivf_entries).
            ((self.0 >> 11) as f32) / ((u64::MAX >> 11) as f32)
        }

        fn unit_vector(&mut self, dim: usize) -> Vec<f32> {
            let mut v: Vec<f32> = (0..dim).map(|_| self.next_f32() * 2.0 - 1.0).collect();
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-10);
            for x in v.iter_mut() {
                *x /= norm;
            }
            v
        }
    }

    /// Clustered dataset: `clusters` Gaussian-ish blobs in `dim`-space.
    /// Returns (entries, held-out queries). Deterministic (fixed seed).
    /// NOTE: clustered on purpose — IVF recall is only meaningful where
    /// clusters exist. Near-orthogonal hash vectors (FastHashEmbedder)
    /// defeat every partition-based ANN; see module docs.
    fn clustered_data(
        clusters: usize,
        per_cluster: usize,
        dim: usize,
        n_queries: usize,
        noise: f32,
    ) -> (HarnessEntries, Vec<Vec<f32>>) {
        let mut rng = XorShift(0x12345678);
        let centers: Vec<Vec<f32>> = (0..clusters).map(|_| rng.unit_vector(dim)).collect();
        let mut entries = Vec::new();
        for (c, center) in centers.iter().enumerate() {
            for m in 0..per_cluster {
                let mut v: Vec<f32> = center
                    .iter()
                    .map(|x| x + (rng.next_f32() * 2.0 - 1.0) * noise)
                    .collect();
                let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-10);
                for x in v.iter_mut() {
                    *x /= norm;
                }
                entries.push((format!("c{c}_m{m}"), v, format!("cluster {c} member {m}")));
            }
        }
        let queries: Vec<Vec<f32>> = (0..n_queries)
            .map(|i| {
                let center = &centers[i % clusters];
                let mut v: Vec<f32> = center
                    .iter()
                    .map(|x| x + (rng.next_f32() * 2.0 - 1.0) * noise)
                    .collect();
                let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-10);
                for x in v.iter_mut() {
                    *x /= norm;
                }
                v
            })
            .collect();
        (entries, queries)
    }

    fn brute_top_k(entries: &[(String, Vec<f32>, String)], query: &[f32], k: usize) -> Vec<String> {
        let mut scored: Vec<(String, f32)> = entries
            .iter()
            .map(|(id, v, _)| (id.clone(), cosine_similarity(query, v)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(k).map(|(id, _)| id).collect()
    }

    fn recall_at_k(index: &IvfIndex, queries: &[Vec<f32>], k: usize) -> f64 {
        let mut hits = 0usize;
        for q in queries {
            let truth = brute_top_k(&index.entries, q, k);
            let got: Vec<String> = index
                .search_by_vector(q, k)
                .into_iter()
                .map(|r| r.id)
                .collect();
            hits += got.iter().filter(|id| truth.contains(id)).count();
        }
        hits as f64 / (queries.len() * k) as f64
    }

    #[test]
    fn ivf_recall_on_clustered_data() {
        // N=2048 (16 clusters x 128), D=64, Q=30, k=10, nprobe=8.
        // Floor 0.80 is a regression tripwire, not a spec: measured
        // recall is recorded by bench_ivf_recall_and_crossover.
        let (entries, queries) = clustered_data(16, 128, 64, 30, 0.15);
        let mut ivf = IvfIndex::new(64);
        ivf.set_nlist(32);
        ivf.set_nprobe(8);
        for (id, v, t) in entries {
            ivf.insert(id, v, t).unwrap();
        }
        ivf.train().unwrap();
        assert!(ivf.is_trained());
        let recall = recall_at_k(&ivf, &queries, 10);
        println!("ivf recall@10 (N=2048, nlist=32, nprobe=8): {recall:.3}");
        assert!(
            recall >= 0.95,
            "IVF recall regressed on clustered data: {recall:.3}"
        );
    }

    /// Recall + latency crossover vs linear. Ignored in normal runs;
    /// execute explicitly and record numbers in docs/BENCHMARKS.md.
    #[test]
    #[ignore]
    fn bench_ivf_recall_and_crossover() {
        for n in [1000usize, 5000, 12000] {
            let per = n / 16;
            let (entries, queries) = clustered_data(16, per, 64, 20, 0.15);
            let mut ivf = IvfIndex::new(64);
            ivf.set_nlist(64);
            ivf.set_nprobe(8);
            for (id, v, t) in &entries {
                ivf.insert(id.clone(), v.clone(), t.clone()).unwrap();
            }
            let t0 = std::time::Instant::now();
            ivf.train().unwrap();
            let train_ms = t0.elapsed().as_millis();
            let recall = recall_at_k(&ivf, &queries, 10);
            let iters = 10;
            let t1 = std::time::Instant::now();
            for q in &queries[..iters.min(queries.len())] {
                let _ = ivf.search_by_vector(q, 10);
            }
            let ivf_us = t1.elapsed().as_micros() / iters as u128;
            // Linear baseline over identical entries.
            let t2 = std::time::Instant::now();
            for q in &queries[..iters.min(queries.len())] {
                let _ = brute_top_k(&entries, q, 10);
            }
            let lin_us = t2.elapsed().as_micros() / iters as u128;
            println!(
                "ivf N={n}: train={train_ms}ms recall@10={recall:.3} \
                 ivf={ivf_us}µs/q linear={lin_us}µs/q"
            );
        }
    }

    /// Diagnostic sweep: which knob moves recall? Ignored; prints a table.
    #[test]
    #[ignore]
    fn bench_ivf_param_sweep() {
        for noise in [0.05f32, 0.15f32] {
            for nlist in [16usize, 32] {
                for nprobe in [4usize, 8, 16, 32] {
                    let (entries, queries) = clustered_data(16, 128, 64, 20, noise);
                    let mut ivf = IvfIndex::new(64);
                    ivf.set_nlist(nlist);
                    ivf.set_nprobe(nprobe);
                    for (id, v, t) in entries {
                        ivf.insert(id, v, t).unwrap();
                    }
                    ivf.train().unwrap();
                    let recall = recall_at_k(&ivf, &queries, 10);
                    println!("noise={noise} nlist={nlist} nprobe={nprobe}: recall@10={recall:.3}");
                }
            }
        }
    }
}
