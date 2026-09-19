use std::sync::Mutex;

use crate::llm_neural_embed::NeuralEmbedder;
use crate::Result;

pub struct EmbeddingModel {
    dimensions: usize,
}

impl EmbeddingModel {
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

impl NeuralEmbedder for EmbeddingModel {
    fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        EmbeddingModel::embed(self, text)
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

pub struct VectorStore {
    embeddings: Vec<(String, Vec<f32>, String)>,
    model: Mutex<Box<dyn NeuralEmbedder>>,
}

impl VectorStore {
    pub fn new(dimensions: usize) -> Self {
        Self {
            embeddings: Vec::new(),
            model: Mutex::new(Box::new(EmbeddingModel::new(dimensions)) as Box<dyn NeuralEmbedder>),
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

        let mut results: Vec<SearchResult> = self
            .embeddings
            .iter()
            .map(|(id, embedding, text)| {
                let score = cosine_similarity(&query_embedding, embedding);
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

        Ok(results)
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

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub id: String,
    pub text: String,
    pub score: f32,
}
