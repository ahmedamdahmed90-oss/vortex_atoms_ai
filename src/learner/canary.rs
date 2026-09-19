// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-02 Section 1: Staged Live Activation — Canary Overlay & Watchdog.
// Canary mode runs a single source with strict limits. Watchdog monitors
// budgets and auto-pauses on violation.

use crate::learner::compliance::seed_sources;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Canary overlay config — loaded via `--phase canary` flag.
/// Does NOT mutate vortex.json.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CanaryConfig {
    pub enabled: bool,
    pub source_id: String,
    pub max_pages: usize,
    pub max_runtime_minutes: u64,
    pub abort_on_allowlist_miss: bool,
}

/// Phase of the staged activation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActivationPhase {
    Canary,
    Beta,
    Full,
}

/// Watchdog state — monitors budgets and auto-pauses.
#[derive(Clone, Debug)]
pub struct Watchdog {
    paused: bool,
    pause_reason: Option<String>,
    embed_queue_depth: usize,
    learner_cpu_pct: f64,
    inference_semaphore_held: bool,
    cpu_threshold_pct: f64,
    queue_threshold: usize,
    runtime_threshold: Duration,
    last_check: Instant,
}

impl Default for Watchdog {
    fn default() -> Self {
        Self {
            paused: false,
            pause_reason: None,
            embed_queue_depth: 0,
            learner_cpu_pct: 0.0,
            inference_semaphore_held: false,
            cpu_threshold_pct: 60.0,
            queue_threshold: 500,
            runtime_threshold: Duration::from_secs(600),
            last_check: Instant::now(),
        }
    }
}

/// Telemetry counters for a single run.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunTelemetry {
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

impl CanaryConfig {
    pub fn canary() -> Self {
        Self {
            enabled: true,
            source_id: String::from("wikipedia_en"),
            max_pages: 25,
            max_runtime_minutes: 5,
            abort_on_allowlist_miss: true,
        }
    }
}

impl Watchdog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(
        &mut self,
        queue_depth: usize,
        cpu_pct: f64,
        inference_held: bool,
        elapsed: Duration,
    ) -> WatchdogStatus {
        if self.paused {
            return WatchdogStatus {
                paused: true,
                reason: self.pause_reason.clone(),
            };
        }

        self.embed_queue_depth = queue_depth;
        self.learner_cpu_pct = cpu_pct;
        self.inference_semaphore_held = inference_held;
        self.last_check = Instant::now();

        if queue_depth > self.queue_threshold && inference_held {
            self.pause("embed_queue_depth_exceeded".to_string());
            return WatchdogStatus {
                paused: true,
                reason: self.pause_reason.clone(),
            };
        }

        if cpu_pct > self.cpu_threshold_pct && inference_held && elapsed > self.runtime_threshold {
            self.pause("learner_cpu_exceeded".to_string());
            return WatchdogStatus {
                paused: true,
                reason: self.pause_reason.clone(),
            };
        }

        WatchdogStatus {
            paused: false,
            reason: None,
        }
    }

    fn pause(&mut self, reason: String) {
        self.paused = true;
        self.pause_reason = Some(reason);
    }

    pub fn resume(&mut self) {
        self.paused = false;
        self.pause_reason = None;
    }
}

/// Watchdog status reported via /v1/health.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchdogStatus {
    pub paused: bool,
    pub reason: Option<String>,
}

/// Abort on allowlist miss — ANY URL outside the allowlist triggers CRITICAL audit.
pub fn check_allowlist_miss(url: &str) -> Result<(), AllowlistMiss> {
    let sources = seed_sources();
    let is_allowed = sources
        .iter()
        .any(|s| url.contains(&s.id) && s.paths.iter().any(|p| url.contains(p)));
    if !is_allowed {
        Err(AllowlistMiss {
            url: url.to_string(),
        })
    } else {
        Ok(())
    }
}

/// Allowlist miss error — triggers CRITICAL audit.
#[derive(Clone, Debug)]
pub struct AllowlistMiss {
    pub url: String,
}

impl std::fmt::Display for AllowlistMiss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CRITICAL: URL outside allowlist: {}", self.url)
    }
}

impl std::error::Error for AllowlistMiss {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canary_config_defaults() {
        let cfg = CanaryConfig::canary();
        assert_eq!(cfg.source_id, "wikipedia_en");
        assert_eq!(cfg.max_pages, 25);
        assert_eq!(cfg.max_runtime_minutes, 5);
        assert!(cfg.abort_on_allowlist_miss);
    }

    #[test]
    fn watchdog_triggers_on_queue_depth() {
        let mut wd = Watchdog::new();
        let status = wd.check(600, 30.0, true, Duration::from_secs(700));
        assert!(status.paused);
        assert!(status.reason.is_some());
    }

    #[test]
    fn watchdog_allows_normal_queue() {
        let mut wd = Watchdog::new();
        let status = wd.check(100, 30.0, true, Duration::from_secs(100));
        assert!(!status.paused);
    }

    #[test]
    fn watchdog_allows_low_cpu() {
        let mut wd = Watchdog::new();
        let status = wd.check(100, 30.0, true, Duration::from_secs(100));
        assert!(!status.paused);
    }

    #[test]
    fn watchdog_allows_without_inference() {
        let mut wd = Watchdog::new();
        let status = wd.check(100, 70.0, false, Duration::from_secs(700));
        assert!(!status.paused);
    }

    #[test]
    fn watchdog_resumes() {
        let mut wd = Watchdog::new();
        wd.check(600, 30.0, true, Duration::from_secs(700));
        assert!(wd.paused);
        wd.resume();
        assert!(!wd.paused);
    }

    #[test]
    fn watchdog_defaults_to_unpaused() {
        let wd = Watchdog::new();
        assert!(!wd.paused);
    }

    #[test]
    fn watchdog_does_not_trigger_without_inference() {
        let mut wd = Watchdog::new();
        let status = wd.check(100, 70.0, false, Duration::from_secs(700));
        assert!(!status.paused);
    }

    #[test]
    fn watchdog_does_not_trigger_with_short_elapsed() {
        let mut wd = Watchdog::new();
        let status = wd.check(100, 70.0, true, Duration::from_secs(100));
        assert!(!status.paused);
    }

    #[test]
    fn allowlist_miss_rejects_external_url() {
        let result = check_allowlist_miss("https://evil.com/page");
        assert!(result.is_err());
    }

    #[test]
    fn allowlist_miss_allows_wikipedia() {
        let result = check_allowlist_miss("https://wikipedia_en/wiki/Main_Page");
        assert!(result.is_ok());
    }

    #[test]
    fn allowlist_miss_allows_mdn() {
        let result = check_allowlist_miss("https://mdn/en-US/docs/Web/JavaScript");
        assert!(result.is_ok());
    }
}
