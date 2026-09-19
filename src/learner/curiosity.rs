// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-01 Section 5: Curiosity Loop.
// Self-directed learning: records retrieval misses, resolves gaps.

use crate::learner::state::{StateStore, GapRecord};

/// Records retrieval misses as curiosity gaps.
pub struct CuriosityEngine {
    state: StateStore,
    max_new_topics_per_run: usize,
}

impl CuriosityEngine {
    pub fn new(state: StateStore, max_new_topics_per_run: usize) -> Self {
        Self {
            state,
            max_new_topics_per_run,
        }
    }

    /// Records a retrieval miss: top score below threshold → normalized topic.
    pub fn record_miss(&self, query: &str, score: f32, threshold: f32) {
        if score < threshold {
            let topic = Self::normalize_topic(query);
            self.state.add_gap(topic);
        }
    }

    /// Picks top-N open gaps for targeted fetch.
    pub fn pick_top_gaps(&self) -> Vec<GapRecord> {
        let gaps = self.state.open_gaps();
        gaps.into_iter().take(self.max_new_topics_per_run).collect()
    }

    /// Checks if a gap is resolved after re-indexing.
    pub fn check_gap_resolved(&self, topic: &str, new_score: f32, threshold: f32) -> bool {
        if new_score >= threshold {
            self.state.resolve_gap(topic);
            true
        } else {
            false
        }
    }

    /// Normalizes a query into a topic string.
    fn normalize_topic(query: &str) -> String {
        let lower = query.to_ascii_lowercase();
        let words: Vec<_> = lower.split_whitespace().take(5).collect();
        words.join(" ")
    }

    /// Returns a search endpoint for an allowlisted source.
    pub fn resolve_source_for_topic(&self, topic: &str) -> Option<&str> {
        if topic.to_ascii_lowercase().contains("rust") {
            Some("rust_doc")
        } else if topic.to_ascii_lowercase().contains("python") {
            Some("python_doc")
        } else {
            Some("wikipedia_en")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_engine() -> CuriosityEngine {
        let state = StateStore::open(&PathBuf::from("/tmp/curiosity_test")).unwrap();
        CuriosityEngine::new(state, 5)
    }

    #[test]
    fn miss_below_threshold_creates_gap() {
        let engine = make_engine();
        engine.record_miss("quantum entanglement", 0.2, 0.5);
        let gaps = engine.state.open_gaps();
        assert!(!gaps.is_empty());
    }

    #[test]
    fn miss_above_threshold_no_gap() {
        let engine = make_engine();
        engine.record_miss("quantum entanglement", 0.8, 0.5);
        let gaps = engine.state.open_gaps();
        assert!(gaps.is_empty());
    }

    #[test]
    fn pick_top_gaps_respects_limit() {
        let engine = make_engine();
        engine.record_miss("topic a", 0.1, 0.5);
        engine.record_miss("topic b", 0.1, 0.5);
        engine.record_miss("topic c", 0.1, 0.5);
        let gaps = engine.pick_top_gaps();
        assert!(gaps.len() <= 5);
    }

    #[test]
    fn check_gap_resolved() {
        let engine = make_engine();
        engine.record_miss("test topic", 0.1, 0.5);
        let gaps = engine.state.open_gaps();
        let topic = gaps[0].topic.clone();
        assert!(engine.check_gap_resolved(&topic, 0.9, 0.5));
        let gaps = engine.state.open_gaps();
        assert!(gaps.is_empty());
    }

    #[test]
    fn check_gap_not_resolved() {
        let engine = make_engine();
        engine.record_miss("test topic", 0.1, 0.5);
        let gaps = engine.state.open_gaps();
        let topic = gaps[0].topic.clone();
        assert!(!engine.check_gap_resolved(&topic, 0.3, 0.5));
    }

    #[test]
    fn resolve_source_for_rust_topic() {
        let engine = make_engine();
        assert_eq!(engine.resolve_source_for_topic("rust ownership"), Some("rust_doc"));
    }

    #[test]
    fn resolve_source_for_python_topic() {
        let engine = make_engine();
        assert_eq!(engine.resolve_source_for_topic("python async"), Some("python_doc"));
    }

    #[test]
    fn resolve_source_for_generic_topic() {
        let engine = make_engine();
        assert_eq!(engine.resolve_source_for_topic("history"), Some("wikipedia_en"));
    }
}
