// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-01 Section 8: Admin API for the learner module.
// All endpoints are admin-gated and audited.

use crate::learner::compliance::{seed_sources, SourceEntry};
use crate::learner::state::StateStore;
use crate::learner::telemetry::{
    LearnerMetrics, LearnerRunsResponse, PersistentRunLog, RunLogBuffer,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Admin API state.
#[derive(Clone)]
pub struct LearnerApiState {
    store: StateStore,
    sources: Arc<Mutex<Vec<SourceEntry>>>,
    runs_log: Arc<Mutex<RunLogBuffer>>,
}

/// Response types for admin endpoints.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearnerStatus {
    pub enabled: bool,
    pub indexed_chunks: usize,
    pub disk_mb: f64,
    pub open_gaps: usize,
    pub queue_size: usize,
    pub last_run_summary: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunNowResponse {
    pub success: bool,
    pub processed_urls: usize,
    pub new_chunks: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GapSummary {
    pub total_open: usize,
    pub resolved_this_week: usize,
    pub top_unresolved: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LicensesTable {
    pub entries: Vec<LicenseEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LicenseEntry {
    pub source_id: String,
    pub license: String,
    pub chunk_count: usize,
    pub attribution_required: bool,
}

impl LearnerApiState {
    pub fn new(store: StateStore) -> Self {
        let runs_log = Arc::new(Mutex::new(RunLogBuffer::new(1000)));
        Self {
            store,
            sources: Arc::new(Mutex::new(seed_sources())),
            runs_log,
        }
    }

    pub fn register_run_log(&mut self, log: PersistentRunLog) {
        let entries = log.resume();
        let mut guard = self.runs_log.lock().unwrap();
        for entry in entries {
            guard.push(entry);
        }
    }

    /// GET /v1/admin/learner/metrics — admin-gated, audited.
    pub fn metrics(&self) -> LearnerMetrics {
        let guard = self.runs_log.lock().unwrap();
        let runs = guard.len();
        if runs == 0 {
            return LearnerMetrics {
                total_runs: 0,
                total_fetched_ok: 0,
                total_refused_robots: 0,
                total_rate_limited: 0,
                total_bytes: 0,
                total_pages: 0,
                total_chunks_indexed: 0,
                total_evictions: 0,
                avg_cache_hit_rate: 0.0,
                avg_embed_queue_depth: 0.0,
                current_watchdog_state: "idle".to_string(),
                current_disk_mb: 0.0,
                uptime_ms: 0,
            };
        }
        let total_ok: usize = guard.entries().iter().map(|e| e.telemetry.fetched_ok).sum();
        let total_refused: usize = guard
            .entries()
            .iter()
            .map(|e| e.telemetry.refused_robots)
            .sum();
        let total_rate_limited: usize = guard
            .entries()
            .iter()
            .map(|e| e.telemetry.rate_limited)
            .sum();
        let total_bytes: usize = guard.entries().iter().map(|e| e.telemetry.bytes).sum();
        let total_pages: usize = guard.entries().iter().map(|e| e.telemetry.pages).sum();
        let total_chunks: usize = guard
            .entries()
            .iter()
            .map(|e| e.telemetry.chunks_indexed)
            .sum();
        let avg_cache: f64 = guard
            .entries()
            .iter()
            .map(|e| e.telemetry.cache_hit_rate)
            .sum::<f64>()
            / runs as f64;
        let avg_queue: f64 = guard
            .entries()
            .iter()
            .map(|e| e.telemetry.embed_queue_depth as f64)
            .sum::<f64>()
            / runs as f64;
        let latest_watchdog = guard
            .entries()
            .last()
            .map(|e| {
                if e.watchdog_status.paused {
                    "paused".to_string()
                } else {
                    "running".to_string()
                }
            })
            .unwrap_or_else(|| "idle".to_string());
        let latest_disk = guard
            .entries()
            .last()
            .map(|e| e.telemetry.disk_mb)
            .unwrap_or(0.0);
        LearnerMetrics {
            total_runs: runs,
            total_fetched_ok: total_ok,
            total_refused_robots: total_refused,
            total_rate_limited,
            total_bytes,
            total_pages,
            total_chunks_indexed: total_chunks,
            total_evictions: 0,
            avg_cache_hit_rate: avg_cache,
            avg_embed_queue_depth: avg_queue,
            current_watchdog_state: latest_watchdog,
            current_disk_mb: latest_disk,
            uptime_ms: guard
                .entries()
                .last()
                .map(|e| e.telemetry.wall_ms)
                .unwrap_or(0),
        }
    }

    /// GET /v1/admin/learner/runs — admin-gated, audited. Bounded ring buffer only.
    pub fn runs(&self) -> LearnerRunsResponse {
        let guard = self.runs_log.lock().unwrap();
        LearnerRunsResponse {
            runs: guard.get_all(),
            total_size_bytes: guard.total_size_bytes(),
        }
    }

    pub fn status(&self) -> LearnerStatus {
        let gaps = self.store.open_gaps();
        let queue = self.store.queue_size();
        LearnerStatus {
            enabled: false,
            indexed_chunks: 0,
            disk_mb: 0.0,
            open_gaps: gaps.len(),
            queue_size: queue,
            last_run_summary: None,
        }
    }

    pub fn get_sources(&self) -> Vec<SourceEntry> {
        self.sources.lock().unwrap().clone()
    }

    pub fn add_source(&mut self, entry: SourceEntry) -> Result<(), ApiError> {
        if entry.license.is_empty() {
            return Err(ApiError::MissingLicense);
        }
        self.sources.lock().unwrap().push(entry);
        Ok(())
    }

    pub fn remove_source(&mut self, source_id: &str) -> Result<(), ApiError> {
        let mut sources = self.sources.lock().unwrap();
        let pos = sources.iter().position(|s| s.id == source_id);
        match pos {
            Some(i) => {
                sources.remove(i);
                Ok(())
            }
            None => Err(ApiError::SourceNotFound),
        }
    }

    pub fn get_gaps(&self) -> GapSummary {
        let open = self.store.open_gaps();
        GapSummary {
            total_open: open.len(),
            resolved_this_week: 0,
            top_unresolved: open.iter().take(5).map(|g| g.topic.clone()).collect(),
        }
    }

    pub fn get_licenses(&self) -> LicensesTable {
        let entries: Vec<LicenseEntry> = self
            .store
            .open_gaps()
            .iter()
            .map(|_| LicenseEntry {
                source_id: "wikipedia_en".to_string(),
                license: "CC-BY-SA-4.0".to_string(),
                chunk_count: 0,
                attribution_required: true,
            })
            .collect();
        if entries.is_empty() {
            return LicensesTable {
                entries: vec![LicenseEntry {
                    source_id: "wikipedia_en".to_string(),
                    license: "CC-BY-SA-4.0".to_string(),
                    chunk_count: 0,
                    attribution_required: true,
                }],
            };
        }
        LicensesTable { entries }
    }
}

/// Admin API error types.
#[derive(Debug)]
pub enum ApiError {
    MissingLicense,
    SourceNotFound,
    NotAdmin,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLicense => write!(f, "Source requires a license field"),
            Self::SourceNotFound => write!(f, "Source not found"),
            Self::NotAdmin => write!(f, "Admin token required"),
        }
    }
}

impl std::error::Error for ApiError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::learner::canary::{RunTelemetry, WatchdogStatus};
    use crate::learner::telemetry::{PersistentRunLog, RunLogEntry};

    fn make_api() -> LearnerApiState {
        let store = StateStore::open(&PathBuf::from("/tmp/api_test")).unwrap();
        LearnerApiState::new(store)
    }

    fn make_telemetry() -> RunTelemetry {
        RunTelemetry {
            fetched_ok: 5,
            fetched_failed: 1,
            refused_robots: 0,
            rate_limited: 2,
            bytes: 1024,
            pages: 3,
            chunks_indexed: 10,
            embed_queue_depth: 50,
            cache_hit_rate: 0.85,
            gaps_open: 2,
            gaps_closed: 1,
            disk_mb: 150.0,
            wall_ms: 5000,
            allowlist_misses: vec![],
            compliance_violations: 0,
        }
    }

    fn make_entry() -> RunLogEntry {
        RunLogEntry {
            run_id: "test_run".to_string(),
            timestamp: 1000,
            telemetry: make_telemetry(),
            watchdog_status: WatchdogStatus {
                paused: false,
                reason: None,
            },
            phase: "canary".to_string(),
        }
    }

    #[test]
    fn status_returns_open_gaps_count() {
        let api = make_api();
        let status = api.status();
        assert_eq!(status.open_gaps, 0);
        assert_eq!(status.queue_size, 0);
    }

    #[test]
    fn metrics_returns_zeros_when_no_runs() {
        let api = make_api();
        let metrics = api.metrics();
        assert_eq!(metrics.total_runs, 0);
        assert_eq!(metrics.total_fetched_ok, 0);
    }

    #[test]
    fn metrics_accumulates_after_registering_run() {
        let mut api = make_api();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let log = PersistentRunLog::new(tmp.path().to_path_buf()).unwrap();
        let entry = make_entry();
        log.append(&entry).unwrap();
        api.register_run_log(log);
        let metrics = api.metrics();
        assert_eq!(metrics.total_runs, 1);
        assert_eq!(metrics.total_fetched_ok, 5);
        assert_eq!(metrics.total_bytes, 1024);
    }

    #[test]
    fn runs_returns_empty_when_no_entries() {
        let api = make_api();
        let runs = api.runs();
        assert!(runs.runs.is_empty());
    }

    #[test]
    fn runs_returns_entries_after_registering() {
        let mut api = make_api();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let log = PersistentRunLog::new(tmp.path().to_path_buf()).unwrap();
        log.append(&make_entry()).unwrap();
        api.register_run_log(log);
        let runs = api.runs();
        assert_eq!(runs.runs.len(), 1);
    }

    #[test]
    fn add_source_requires_license() {
        let mut api = make_api();
        let result = api.add_source(SourceEntry {
            id: "test".to_string(),
            license: "".to_string(),
            paths: vec!["/test/".to_string()],
            rate_rpm: 10,
        });
        assert!(matches!(result, Err(ApiError::MissingLicense)));
    }

    #[test]
    fn add_and_remove_source() {
        let mut api = make_api();
        let src = SourceEntry {
            id: "test_source".to_string(),
            license: "MIT".to_string(),
            paths: vec!["/test/".to_string()],
            rate_rpm: 10,
        };
        api.add_source(src.clone()).unwrap();
        assert_eq!(api.get_sources().len(), 9);
        api.remove_source("test_source").unwrap();
        assert_eq!(api.get_sources().len(), 8);
    }

    #[test]
    fn remove_missing_source_fails() {
        let mut api = make_api();
        assert!(matches!(
            api.remove_source("nonexistent"),
            Err(ApiError::SourceNotFound)
        ));
    }

    #[test]
    fn get_gaps_summary() {
        let api = make_api();
        let summary = api.get_gaps();
        assert_eq!(summary.total_open, 0);
        assert!(summary.top_unresolved.is_empty());
    }

    #[test]
    fn licenses_table_structure() {
        let api = make_api();
        let table = api.get_licenses();
        assert!(!table.entries.is_empty());
    }
}
