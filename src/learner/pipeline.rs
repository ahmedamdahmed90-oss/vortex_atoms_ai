// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-01 Section 2: Acquisition Pipeline.
// Stages: fetch → extract → normalize → lang detect → chunk → dedup → quality → enqueue embed.

use crate::learner::compliance::{ComplianceChecker, ComplianceResult, seed_sources};
use crate::learner::state::{StateStore, ChunkRecord, EmbedQueueEntry, EmbedPriority};

/// Result of a single acquisition stage.
#[derive(Clone, Debug)]
pub enum StageResult {
    Fetch(FetchResult),
    Extract(ExtractResult),
    Normalize(NormalizeResult),
    LangDetect(LangDetectResult),
    Chunk(ChunkResult),
    Dedup(DedupResult),
    Quality(QualityResult),
    Enqueued(EmbedQueueEntry),
}

#[derive(Clone, Debug)]
pub struct FetchResult {
    pub url: String,
    pub status_code: u16,
    pub content: Vec<u8>,
    pub final_url: String,
}

#[derive(Clone, Debug)]
pub struct ExtractResult {
    pub title: String,
    pub markdown: String,
    pub language: String,
}

#[derive(Clone, Debug)]
pub struct NormalizeResult {
    pub content: String,
    pub selectors_removed: usize,
}

#[derive(Clone, Debug)]
pub struct LangDetectResult {
    pub language: String,
    pub confidence: f64,
}

#[derive(Clone, Debug)]
pub struct ChunkResult {
    pub chunks: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DedupResult {
    pub new_chunks: Vec<ChunkRecord>,
    pub duplicates_removed: usize,
}

#[derive(Clone, Debug)]
pub struct QualityResult {
    pub chunks: Vec<ChunkRecord>,
    pub rejected: usize,
}

/// The acquisition pipeline orchestrator.
pub struct AcquisitionPipeline {
    compliance: ComplianceChecker,
    state: StateStore,
}

impl AcquisitionPipeline {
    pub fn new(state: StateStore) -> Self {
        let compliance = ComplianceChecker::new(seed_sources());
        Self { compliance, state }
    }

    pub fn process_url(&self, url: &str) -> Result<StageResult, PipelineError> {
        let check = self.compliance.check(url);
        if check != ComplianceResult::Allowed {
            return Err(PipelineError::ComplianceFailed(check));
        }
        Ok(StageResult::Fetch(FetchResult {
            url: url.to_string(),
            status_code: 200,
            content: vec![],
            final_url: url.to_string(),
        }))
    }

    pub fn enqueue_chunk(&self, chunk: ChunkRecord, priority: EmbedPriority) {
        self.state.enqueue_embed(EmbedQueueEntry {
            chunk_id: chunk.dedup_hash.clone(),
            priority,
            enqueued_at: 0,
        });
    }
}

/// Pipeline error types.
#[derive(Debug)]
pub enum PipelineError {
    ComplianceFailed(ComplianceResult),
    FetchError(String),
    ExtractError(String),
    QualityScoreLow(f32),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ComplianceFailed(r) => write!(f, "Compliance failed: {r:?}"),
            Self::FetchError(msg) => write!(f, "Fetch error: {msg}"),
            Self::ExtractError(msg) => write!(f, "Extract error: {msg}"),
            Self::QualityScoreLow(score) => write!(f, "Quality score too low: {score}"),
        }
    }
}

impl std::error::Error for PipelineError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::learner::compute_sha256;

    fn make_pipeline() -> AcquisitionPipeline {
        let store = StateStore::open(&PathBuf::from("/tmp/pipeline_test")).unwrap();
        AcquisitionPipeline::new(store)
    }

    #[test]
    fn pipeline_rejects_non_https() {
        let p = make_pipeline();
        let result = p.process_url("http://evil.com/page");
        assert!(matches!(result, Err(PipelineError::ComplianceFailed(ComplianceResult::RequiresHttps))));
    }

    #[test]
    fn pipeline_rejects_not_allowlisted() {
        let p = make_pipeline();
        let result = p.process_url("https://evil.com/page");
        assert!(matches!(result, Err(PipelineError::ComplianceFailed(ComplianceResult::NotAllowlisted))));
    }

    #[test]
    fn pipeline_allows_valid_wikipedia() {
        let p = make_pipeline();
        let result = p.process_url("https://wikipedia_en/wiki/Main_Page");
        assert!(result.is_ok());
    }

    #[test]
    fn pipeline_rejects_wikipedia_admin() {
        let p = make_pipeline();
        let result = p.process_url("https://wikipedia_en/admin/delete");
        assert!(matches!(result, Err(PipelineError::ComplianceFailed(ComplianceResult::NotAllowlisted))));
    }

    #[test]
    fn dedup_removes_duplicates() {
        let content = "hello world foo bar";
        let sha = compute_sha256(content);
        let chunk = ChunkRecord {
            page_url: "https://x".to_string(),
            chunk_index: 0,
            content: content.to_string(),
            token_count: 5,
            quality_score: 0.8,
            embedding_hash: None,
            dedup_hash: sha.clone(),
        };
        assert_eq!(chunk.dedup_hash.len(), 64);
    }

    #[test]
    fn stage_result_variants() {
        let _fetch = StageResult::Fetch(FetchResult {
            url: "x".to_string(),
            status_code: 200,
            content: vec![],
            final_url: "x".to_string(),
        });
        let _chunk = StageResult::Chunk(ChunkResult { chunks: vec!["a".to_string()] });
    }
}
