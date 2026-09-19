// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-01 Section 3: Embeddings Queue & Cache.
// Batch=1, priority queue, CPU guard yielding to inference semaphore.

use crate::learner::state::EmbedPriority;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};

/// Embedding priority queue entry for the binary heap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueItem {
    pub priority: i8,
    pub chunk_id: String,
    pub enqueued_at: u128,
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .reverse()
            .then_with(|| self.enqueued_at.cmp(&other.enqueued_at))
    }
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Embeddings queue with priority ordering and cache.
#[allow(dead_code)]
pub struct EmbeddingsQueue {
    queue: BinaryHeap<QueueItem>,
    cache: Arc<Mutex<Cache>>,
}

/// Simple embedding cache: hash → vec persisted in memory.
#[derive(Clone, Debug, Default)]
struct Cache {
    entries: std::collections::HashMap<String, Vec<f32>>,
}

impl Default for EmbeddingsQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingsQueue {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            cache: Arc::new(Mutex::new(Cache::default())),
        }
    }

    /// Enqueues a chunk with the given priority.
    pub fn enqueue(&mut self, item: QueueItem) {
        self.queue.push(item);
    }

    /// Dequeues the highest-priority item.
    pub fn dequeue(&mut self) -> Option<QueueItem> {
        self.queue.pop()
    }

    /// Returns the queue size.
    pub fn size(&self) -> usize {
        self.queue.len()
    }

    /// Checks if an embedding is cached.
    pub fn cache_hit(&self, hash: &str) -> bool {
        let guard = self.cache.lock().unwrap();
        guard.entries.contains_key(hash)
    }

    /// Stores an embedding in the cache.
    pub fn cache_embedding(&self, hash: String, vec: Vec<f32>) {
        let mut guard = self.cache.lock().unwrap();
        guard.entries.insert(hash, vec);
    }

    /// Retrieves a cached embedding.
    pub fn get_cached(&self, hash: &str) -> Option<Vec<f32>> {
        let guard = self.cache.lock().unwrap();
        guard.entries.get(hash).cloned()
    }

    /// Converts a priority enum to the internal i8 priority.
    /// (1) curiosity-gap, (2) miss-requeue, (3) fresh page.
    pub fn priority_to_i8(priority: EmbedPriority) -> i8 {
        match priority {
            EmbedPriority::CuriosityGap => 1,
            EmbedPriority::MissRequeue => 2,
            EmbedPriority::FreshPage => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_queue_orders_curiosity_first() {
        let mut q = EmbeddingsQueue::new();
        q.enqueue(QueueItem {
            priority: EmbeddingsQueue::priority_to_i8(
                crate::learner::state::EmbedPriority::FreshPage,
            ),
            chunk_id: "fresh".to_string(),
            enqueued_at: 100,
        });
        q.enqueue(QueueItem {
            priority: EmbeddingsQueue::priority_to_i8(
                crate::learner::state::EmbedPriority::CuriosityGap,
            ),
            chunk_id: "curiosity".to_string(),
            enqueued_at: 50,
        });
        let first = q.dequeue().unwrap();
        assert_eq!(first.chunk_id, "curiosity");
    }

    #[test]
    fn cache_hit_misses() {
        let q = EmbeddingsQueue::new();
        assert!(!q.cache_hit("missing"));
    }

    #[test]
    fn cache_hit_stores_and_retrieves() {
        let q = EmbeddingsQueue::new();
        q.cache_embedding("hash1".to_string(), vec![0.1, 0.2, 0.3]);
        assert!(q.cache_hit("hash1"));
        let vec = q.get_cached("hash1").unwrap();
        assert_eq!(vec, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn priority_values() {
        assert_eq!(
            EmbeddingsQueue::priority_to_i8(EmbedPriority::CuriosityGap),
            1
        );
        assert_eq!(
            EmbeddingsQueue::priority_to_i8(EmbedPriority::MissRequeue),
            2
        );
        assert_eq!(EmbeddingsQueue::priority_to_i8(EmbedPriority::FreshPage), 3);
    }

    #[test]
    fn queue_size_tracking() {
        let mut q = EmbeddingsQueue::new();
        assert_eq!(q.size(), 0);
        q.enqueue(QueueItem {
            priority: 1,
            chunk_id: "c1".to_string(),
            enqueued_at: 0,
        });
        assert_eq!(q.size(), 1);
    }
}
