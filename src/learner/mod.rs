// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// Vortex Atoms AI — KNOW-01 Knowledge Acquisition System.
// This module implements the autonomous knowledge acquisition pipeline.
// Sources are allowlist-only. No URL outside the registry is ever fetched.
// See docs/LEARNER.md for the honesty clause and compliance details.

pub mod canary;
pub mod compliance;
pub mod pipeline;
pub mod state;
pub mod curiosity;
pub mod compile;
pub mod api;
pub mod scheduler;
pub mod embeddings;
pub mod telemetry;
pub mod eval;
pub mod arabic;
pub mod governor;
pub mod report;

pub use crate::learner::canary::{CanaryConfig, ActivationPhase, Watchdog, WatchdogStatus, RunTelemetry, check_allowlist_miss, AllowlistMiss};
pub use crate::learner::compliance::{ComplianceChecker, ComplianceResult, LicenseRecord, seed_sources, SourceEntry};
pub use crate::learner::state::{StateStore, UrlRecord, PageRecord, ChunkRecord, EmbedQueueEntry, EmbedPriority, compute_sha256, minhash_similarity};
pub use crate::learner::pipeline::AcquisitionPipeline;
pub use crate::learner::curiosity::CuriosityEngine;
pub use crate::learner::compile::FragmentCompiler;
pub use crate::learner::api::{LearnerApiState, LearnerStatus, RunNowResponse, GapSummary, LicensesTable, LicenseEntry};
pub use crate::learner::scheduler::{LearnerScheduler, SchedulerConfig};
pub use crate::learner::embeddings::EmbeddingsQueue;
pub use crate::learner::telemetry::{RunLogBuffer, PersistentRunLog, RunLogEntry, LearnerMetrics, LearnerRunsResponse, TelemetryAccumulator};
pub use crate::learner::eval::{EvalMetrics, EvalRun, RegressionResult, RetrievalResult, GoldenQuery, run_eval, check_regression, run_injection_probes, regression_banner};
pub use crate::learner::arabic::{normalize_arabic, ArabicStemmer, LanguageWeights, FusionWeights, language_aware_fusion, arabic_seed_sources};
pub use crate::learner::governor::{BudgetConfig, BudgetGovernor, DegradeLevel, KillSwitchState, SbomReport, TokenBucket, EvictionAudit, degrade_order_description};
pub use crate::learner::report::{ReportGenerator, AutomatedReport, BudgetSummary, ReportPeriod};
pub use crate::learner::canary::{CanaryConfig as CanaryOverlay, ActivationPhase as Phase};

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub type ScheduleConfig = crate::learner::scheduler::SchedulerConfig;

/// The learner module is compiled into release SKUs but RUNTIME-OFF by default.
/// Enable via vortex.json: learner.enabled = true (requires admin token).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearnerConfig {
    pub enabled: bool,
    pub schedule: ScheduleConfig,
    pub disk_budget_gb: f64,
    pub chunk_cap: usize,
    pub curiosity: CuriosityConfig,
    pub sources: Vec<SourceEntry>,
}

impl Default for LearnerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            schedule: ScheduleConfig::default(),
            disk_budget_gb: 2.0,
            chunk_cap: 200000,
            curiosity: CuriosityConfig::default(),
            sources: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CuriosityConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub max_new_topics_per_run: usize,
}

/// Returns the default local app data path for the learner state database.
pub fn learner_state_path() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("vortex_atoms_ai").join("learner").join("state.db"))
        .unwrap_or_else(|_| PathBuf::from("./learner-state.db"))
}

/// Returns the default directory for learner model downloads.
pub fn learner_models_dir() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("vortex_atoms_ai").join("learner").join("models"))
        .unwrap_or_else(|_| PathBuf::from("./learner-models"))
}

/// Returns the path to the learner fixture directory for tests.
pub fn learner_fixture_dir() -> PathBuf {
    PathBuf::from("tests/fixtures/learner")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_learner_config_is_disabled() {
        let cfg: LearnerConfig = LearnerConfig::default();
        assert!(!cfg.enabled);
        assert!(!cfg.curiosity.enabled);
        assert_eq!(cfg.schedule.run_every_hours, 6);
        assert_eq!(cfg.disk_budget_gb, 2.0);
        assert_eq!(cfg.chunk_cap, 200000);
    }

    #[test]
    fn learner_state_path_ends_with_state_db() {
        let p = learner_state_path();
        assert!(p.to_string_lossy().ends_with("state.db"));
    }

    #[test]
    fn learner_models_dir_ends_with_models() {
        let p = learner_models_dir();
        assert!(p.to_string_lossy().ends_with("models"));
    }

    #[test]
    fn fixture_dir_exists() {
        let p = learner_fixture_dir();
        assert!(p.exists(), "fixture dir should exist");
    }
}
