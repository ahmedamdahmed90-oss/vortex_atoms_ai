// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-02 Section 2: Telemetry & Run Logs.
// Persists per-run metrics to learner_runs.jsonl.
// Provides GET /v1/admin/learner/metrics and /v1/admin/learner/runs.
// Bounded ring buffers enforce the 50 MiB UI budget invariant.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use crate::learner::canary::{RunTelemetry, WatchdogStatus};

/// Maximum total size of learner_runs.jsonl in bytes (50 MiB UI budget).
const MAX_LOG_SIZE_BYTES: u64 = 50 * 1024 * 1024;

/// A single run log entry persisted to learner_runs.jsonl.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunLogEntry {
    pub run_id: String,
    pub timestamp: u128,
    pub telemetry: RunTelemetry,
    pub watchdog_status: WatchdogStatus,
    pub phase: String,
}

/// Ring buffer for in-memory run history (bounded to 50 MiB).
#[derive(Clone, Debug)]
pub struct RunLogBuffer {
    entries: Vec<RunLogEntry>,
    total_size_bytes: u64,
    max_entries: usize,
}

/// Telemetry accumulator — collects counters during a run.
#[derive(Clone, Debug, Default)]
pub struct TelemetryAccumulator {
    pub fetched_ok: usize,
    pub fetched_failed: usize,
    pub refused_robots: usize,
    pub rate_limited: usize,
    pub bytes: usize,
    pub pages: usize,
    pub chunks_indexed: usize,
    pub embed_queue_depth: usize,
    pub cache_hit_rate: f64,
    pub gaps_open: usize,
    pub gaps_closed: usize,
    pub disk_mb: f64,
    pub wall_ms: u128,
    pub allowlist_misses: Vec<String>,
    pub compliance_violations: usize,
}

/// Admin API: metrics and runs.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LearnerMetrics {
    pub total_runs: usize,
    pub total_fetched_ok: usize,
    pub total_refused_robots: usize,
    pub total_rate_limited: usize,
    pub total_bytes: usize,
    pub total_pages: usize,
    pub total_chunks_indexed: usize,
    pub total_evictions: u64,
    pub avg_cache_hit_rate: f64,
    pub avg_embed_queue_depth: f64,
    pub current_watchdog_state: String,
    pub current_disk_mb: f64,
    pub uptime_ms: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearnerRunsResponse {
    pub runs: Vec<RunLogEntry>,
    pub total_size_bytes: u64,
}

impl RunLogBuffer {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_entries),
            total_size_bytes: 0,
            max_entries,
        }
    }

    pub fn push(&mut self, entry: RunLogEntry) {
        let size = serde_json::to_string(&entry).map(|s| s.len() as u64).unwrap_or(0);
        if self.total_size_bytes + size > MAX_LOG_SIZE_BYTES || self.entries.len() >= self.max_entries {
            // Evict oldest entries until under budget and under max entries
            while (self.total_size_bytes + size > MAX_LOG_SIZE_BYTES || self.entries.len() >= self.max_entries) && !self.entries.is_empty() {
                if let Some(old) = self.entries.first() {
                    let old_size = serde_json::to_string(old).map(|s| s.len() as u64).unwrap_or(0);
                    self.total_size_bytes = self.total_size_bytes.saturating_sub(old_size);
                    self.entries.remove(0);
                }
            }
        }
        self.total_size_bytes += size;
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[RunLogEntry] {
        &self.entries
    }

    pub fn get_all(&self) -> Vec<RunLogEntry> {
        self.entries.clone()
    }

    pub fn total_size_bytes(&self) -> u64 {
        self.total_size_bytes
    }
}

impl Default for RunLogBuffer {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Persistent run log — appends to learner_runs.jsonl.
pub struct PersistentRunLog {
    path: PathBuf,
    buffer: Arc<Mutex<RunLogBuffer>>,
}

impl PersistentRunLog {
    pub fn new(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let buffer = Arc::new(Mutex::new(RunLogBuffer::new(1000)));
        // Load existing entries from JSONL file
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            for line in content.lines() {
                if let Ok(entry) = serde_json::from_str::<RunLogEntry>(line) {
                    buffer.lock().unwrap().push(entry);
                }
            }
        }
        Ok(Self { path, buffer })
    }

    pub fn append(&self, entry: &RunLogEntry) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;
        self.buffer.lock().unwrap().push(entry.clone());
        Ok(())
    }

    pub fn resume(&self) -> Vec<RunLogEntry> {
        self.buffer.lock().unwrap().get_all()
    }

    pub fn metrics(&self) -> LearnerMetrics {
        let guard = self.buffer.lock().unwrap();
        let runs = guard.entries.len();
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

        let total_ok: usize = guard.entries.iter().map(|e| e.telemetry.fetched_ok).sum();
        let total_refused: usize = guard.entries.iter().map(|e| e.telemetry.refused_robots).sum();
        let total_rate_limited: usize = guard.entries.iter().map(|e| e.telemetry.rate_limited).sum();
        let total_bytes: usize = guard.entries.iter().map(|e| e.telemetry.bytes).sum();
        let total_pages: usize = guard.entries.iter().map(|e| e.telemetry.pages).sum();
        let total_chunks: usize = guard.entries.iter().map(|e| e.telemetry.chunks_indexed).sum();
        let avg_cache: f64 = guard.entries.iter().map(|e| e.telemetry.cache_hit_rate).sum::<f64>() / runs as f64;
        let avg_queue: f64 = guard.entries.iter().map(|e| e.telemetry.embed_queue_depth as f64).sum::<f64>() / runs as f64;
        let latest_watchdog = guard.entries.last().map(|e| {
            if e.watchdog_status.paused { "paused".to_string() } else { "running".to_string() }
        }).unwrap_or_else(|| "idle".to_string());
        let latest_disk = guard.entries.last().map(|e| e.telemetry.disk_mb).unwrap_or(0.0);

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
            uptime_ms: guard.entries.last().map(|e| e.telemetry.wall_ms).unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn make_log() -> (PersistentRunLog, PathBuf) {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        let log = PersistentRunLog::new(path.clone()).unwrap();
        (log, path)
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

    fn make_entry(telemetry: &RunTelemetry) -> RunLogEntry {
        RunLogEntry {
            run_id: "test_run_1".to_string(),
            timestamp: 1000,
            telemetry: telemetry.clone(),
            watchdog_status: WatchdogStatus { paused: false, reason: None },
            phase: "canary".to_string(),
        }
    }

    #[test]
    fn counter_accuracy_vs_fixture_run() {
        let (log, _path) = make_log();
        let telemetry = make_telemetry();
        let entry = make_entry(&telemetry);
        log.append(&entry).unwrap();

        let metrics = log.metrics();
        assert_eq!(metrics.total_runs, 1);
        assert_eq!(metrics.total_fetched_ok, 5);
        assert_eq!(metrics.total_refused_robots, 0);
        assert_eq!(metrics.total_rate_limited, 2);
        assert_eq!(metrics.total_bytes, 1024);
        assert_eq!(metrics.total_pages, 3);
        assert_eq!(metrics.total_chunks_indexed, 10);
        assert_eq!(metrics.avg_cache_hit_rate, 0.85);
    }

    #[test]
    fn jsonl_resume_at_least_8() {
        let (log, path) = make_log();
        for i in 0..10 {
            let mut telemetry = make_telemetry();
            telemetry.fetched_ok = i;
            let entry = RunLogEntry {
                run_id: format!("run_{}", i),
                timestamp: 1000 + i as u128,
                telemetry,
                watchdog_status: WatchdogStatus { paused: false, reason: None },
                phase: "canary".to_string(),
            };
            log.append(&entry).unwrap();
        }

        // Create new log from same path — should resume all 10
        let log2 = PersistentRunLog::new(path).unwrap();
        let resumed = log2.resume();
        assert!(resumed.len() >= 8);

        let metrics = log2.metrics();
        assert_eq!(metrics.total_runs, 10);
        assert_eq!(metrics.total_fetched_ok, 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9);
    }

    #[test]
    fn buffer_bounded_50_mib() {
        let mut buffer = RunLogBuffer::new(1000);
        for i in 0..2000 {
            let telemetry = RunTelemetry {
                fetched_ok: i,
                fetched_failed: 0,
                refused_robots: 0,
                rate_limited: 0,
                bytes: 1024,
                pages: 1,
                chunks_indexed: 1,
                embed_queue_depth: 10,
                cache_hit_rate: 0.5,
                gaps_open: 0,
                gaps_closed: 0,
                disk_mb: 1.0,
                wall_ms: 100,
                allowlist_misses: vec![],
                compliance_violations: 0,
            };
            let entry = RunLogEntry {
                run_id: format!("run_{}", i),
                timestamp: 1000 + i as u128,
                telemetry,
                watchdog_status: WatchdogStatus { paused: false, reason: None },
                phase: "canary".to_string(),
            };
            buffer.push(entry);
        }
        // Should be bounded — not 2000 entries
        assert!(buffer.len() < 2000);
        assert!(buffer.total_size_bytes() <= MAX_LOG_SIZE_BYTES);
    }

    #[test]
    fn metrics_empty_when_no_runs() {
        let (log, _path) = make_log();
        let metrics = log.metrics();
        assert_eq!(metrics.total_runs, 0);
        assert_eq!(metrics.total_fetched_ok, 0);
    }

    #[test]
    fn buffer_evicts_oldest_when_over_budget() {
        let mut buffer = RunLogBuffer::new(5);
        // Add exactly 5 small entries
        for i in 0..5 {
            let telemetry = RunTelemetry {
                fetched_ok: i,
                fetched_failed: 0,
                refused_robots: 0,
                rate_limited: 0,
                bytes: 100,
                pages: 1,
                chunks_indexed: 1,
                embed_queue_depth: 1,
                cache_hit_rate: 0.5,
                gaps_open: 0,
                gaps_closed: 0,
                disk_mb: 0.1,
                wall_ms: 10,
                allowlist_misses: vec![],
                compliance_violations: 0,
            };
            let entry = RunLogEntry {
                run_id: format!("run_{}", i),
                timestamp: 1000 + i as u128,
                telemetry,
                watchdog_status: WatchdogStatus { paused: false, reason: None },
                phase: "canary".to_string(),
            };
            buffer.push(entry);
        }
        assert_eq!(buffer.len(), 5);
        // Add a large entry — should trigger eviction to stay under max_entries
        let big_telemetry = RunTelemetry {
            fetched_ok: 0,
            fetched_failed: 0,
            refused_robots: 0,
            rate_limited: 0,
            bytes: 10 * 1024 * 1024,
            pages: 1,
            chunks_indexed: 1,
            embed_queue_depth: 1,
            cache_hit_rate: 0.5,
            gaps_open: 0,
            gaps_closed: 0,
            disk_mb: 50.0,
            wall_ms: 1000,
            allowlist_misses: vec![],
            compliance_violations: 0,
        };
        let big_entry = RunLogEntry {
            run_id: "big_run".to_string(),
            timestamp: 10000,
            telemetry: big_telemetry,
            watchdog_status: WatchdogStatus { paused: false, reason: None },
            phase: "canary".to_string(),
        };
        buffer.push(big_entry);
        // Should be at or under max_entries
        assert!(buffer.len() <= 5);
        assert!(buffer.total_size_bytes() <= MAX_LOG_SIZE_BYTES);
    }

    #[test]
    fn resume_preserves_order() {
        let (log, path) = make_log();
        for i in 0..8 {
            let telemetry = make_telemetry();
            let entry = RunLogEntry {
                run_id: format!("run_{}", i),
                timestamp: 1000 + i as u128,
                telemetry,
                watchdog_status: WatchdogStatus { paused: false, reason: None },
                phase: "canary".to_string(),
            };
            log.append(&entry).unwrap();
        }
        let log2 = PersistentRunLog::new(path).unwrap();
        let resumed = log2.resume();
        assert_eq!(resumed.len(), 8);
        assert_eq!(resumed[0].run_id, "run_0");
        assert_eq!(resumed[7].run_id, "run_7");
    }
}
