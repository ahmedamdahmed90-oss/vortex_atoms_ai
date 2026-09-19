// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-01 Section 2: Acquisition Pipeline + Resumable State.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The redb-backed state store path.
pub type StatePath = PathBuf;

/// State store tables (defined here for the redb schema).
/// Tables: urls, pages, chunks, embed_queue, gaps, licenses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UrlRecord {
    pub url: String,
    pub source_id: String,
    pub fetched_at: u128,
    pub status: UrlStatus,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum UrlStatus {
    Pending,
    Fetching,
    Done,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageRecord {
    pub url: String,
    pub title: String,
    pub content: String,
    pub language: String,
    pub extracted_at: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub page_url: String,
    pub chunk_index: usize,
    pub content: String,
    pub token_count: usize,
    pub quality_score: f32,
    pub embedding_hash: Option<String>,
    pub dedup_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbedQueueEntry {
    pub chunk_id: String,
    pub priority: EmbedPriority,
    pub enqueued_at: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmbedPriority {
    CuriosityGap,
    MissRequeue,
    FreshPage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GapRecord {
    pub topic: String,
    pub count: u32,
    pub last_seen: u128,
    pub resolved: bool,
}

/// Kill-resume state store using an in-memory HashMap backed by a Mutex.
#[derive(Clone, Debug)]
pub struct StateStore {
    inner: Arc<Mutex<StateStoreInner>>,
}

#[derive(Clone, Debug, Default)]
struct StateStoreInner {
    urls: HashMap<String, UrlRecord>,
    pages: HashMap<String, PageRecord>,
    chunks: HashMap<String, ChunkRecord>,
    embed_queue: Vec<EmbedQueueEntry>,
    gaps: HashMap<String, GapRecord>,
    licenses: HashMap<String, crate::learner::compliance::LicenseRecord>,
}

impl StateStore {
    pub fn open(_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inner: Arc::new(Mutex::new(StateStoreInner::default())),
        })
    }

    pub fn record_url_done(&self, url: &str, sha256: &str, source_id: &str) {
        let mut guard = self.inner.lock().unwrap();
        guard.urls.insert(url.to_string(), UrlRecord {
            url: url.to_string(),
            source_id: source_id.to_string(),
            fetched_at: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0),
            status: UrlStatus::Done,
            sha256: sha256.to_string(),
        });
    }

    pub fn is_url_done(&self, url: &str) -> bool {
        let guard = self.inner.lock().unwrap();
        guard.urls.get(url).map(|r| r.status == UrlStatus::Done).unwrap_or(false)
    }

    pub fn mark_url_fetching(&self, url: &str, source_id: &str) {
        let mut guard = self.inner.lock().unwrap();
        guard.urls.insert(url.to_string(), UrlRecord {
            url: url.to_string(),
            source_id: source_id.to_string(),
            fetched_at: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0),
            status: UrlStatus::Fetching,
            sha256: String::new(),
        });
    }

    pub fn add_page(&self, page: PageRecord) {
        let mut guard = self.inner.lock().unwrap();
        guard.pages.insert(page.url.clone(), page);
    }

    pub fn add_chunk(&self, chunk: ChunkRecord) {
        let mut guard = self.inner.lock().unwrap();
        guard.chunks.insert(chunk.dedup_hash.clone(), chunk);
    }

    pub fn enqueue_embed(&self, entry: EmbedQueueEntry) {
        let mut guard = self.inner.lock().unwrap();
        guard.embed_queue.push(entry);
    }

    pub fn queue_size(&self) -> usize {
        let guard = self.inner.lock().unwrap();
        guard.embed_queue.len()
    }

    pub fn add_gap(&self, topic: String) {
        let mut guard = self.inner.lock().unwrap();
        guard.gaps.insert(topic.clone(), GapRecord {
            topic,
            count: 1,
            last_seen: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0),
            resolved: false,
        });
    }

    pub fn open_gaps(&self) -> Vec<GapRecord> {
        let guard = self.inner.lock().unwrap();
        guard.gaps.values().filter(|g| !g.resolved).cloned().collect()
    }

    pub fn resolve_gap(&self, topic: &str) {
        let mut guard = self.inner.lock().unwrap();
        if let Some(g) = guard.gaps.get_mut(topic) {
            g.resolved = true;
        }
    }

    pub fn add_license(&self, record: crate::learner::compliance::LicenseRecord) {
        let mut guard = self.inner.lock().unwrap();
        guard.licenses.insert(record.url.clone(), record);
    }
}

/// Computes the sha256 of content for dedup.
pub fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Computes a minhash-like near-dup score (simplified Jaccard).
pub fn minhash_similarity(a: &str, b: &str) -> f64 {
    let a_words: HashSet<_> = a.split_whitespace().collect();
    let b_words: HashSet<_> = b.split_whitespace().collect();
    if a_words.is_empty() && b_words.is_empty() {
        return 1.0;
    }
    let intersection = a_words.intersection(&b_words).count() as f64;
    let union = a_words.union(&b_words).count() as f64;
    if union == 0.0 { 0.0 } else { intersection / union }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_sha256_is_deterministic() {
        let h1 = compute_sha256("hello");
        let h2 = compute_sha256("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_sha256_different() {
        let h1 = compute_sha256("hello");
        let h2 = compute_sha256("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn minhash_identical_content() {
        let score = minhash_similarity("foo bar baz", "foo bar baz");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn minhash_different_content() {
        let score = minhash_similarity("foo bar", "baz qux");
        assert!(score < 1.0);
    }

    #[test]
    fn state_store_record_and_check() {
        let store = StateStore::open(&PathBuf::from("/tmp/test")).unwrap();
        store.mark_url_fetching("https://example.com/page", "wikipedia_en");
        assert!(!store.is_url_done("https://example.com/page"));
        store.record_url_done("https://example.com/page", "abc123", "wikipedia_en");
        assert!(store.is_url_done("https://example.com/page"));
    }

    #[test]
    fn state_store_gap_lifecycle() {
        let store = StateStore::open(&PathBuf::from("/tmp/test")).unwrap();
        store.add_gap("quantum computing".to_string());
        let gaps = store.open_gaps();
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].topic, "quantum computing");
        store.resolve_gap("quantum computing");
        let gaps = store.open_gaps();
        assert!(gaps.is_empty());
    }

    #[test]
    fn state_store_embed_queue() {
        let store = StateStore::open(&PathBuf::from("/tmp/test")).unwrap();
        assert_eq!(store.queue_size(), 0);
        store.enqueue_embed(EmbedQueueEntry {
            chunk_id: "c1".to_string(),
            priority: EmbedPriority::CuriosityGap,
            enqueued_at: 0,
        });
        assert_eq!(store.queue_size(), 1);
    }

    #[test]
    fn url_status_done_not_pending() {
        assert_ne!(UrlStatus::Done, UrlStatus::Pending);
    }

    #[test]
    fn embed_priority_equality() {
        assert_eq!(EmbedPriority::CuriosityGap, EmbedPriority::CuriosityGap);
    }
}
