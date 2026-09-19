// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// Vortex Atoms AI — KNOW-01 Knowledge Acquisition System.
// This module implements the autonomous knowledge acquisition pipeline.
// Sources are allowlist-only. No URL outside the registry is ever fetched.
// See docs/LEARNER.md for the honesty clause and compliance details.

pub mod api;
pub mod arabic;
pub mod canary;
pub mod compile;
pub mod compliance;
pub mod curiosity;
pub mod embeddings;
pub mod eval;
pub mod governor;
pub mod pipeline;
pub mod report;
pub mod scheduler;
pub mod state;
pub mod telemetry;

pub use crate::learner::api::{
    GapSummary, LearnerApiState, LearnerStatus, LicenseEntry, LicensesTable, RunNowResponse,
};
pub use crate::learner::arabic::{
    arabic_seed_sources, language_aware_fusion, normalize_arabic, ArabicStemmer, FusionWeights,
    LanguageWeights,
};
pub use crate::learner::canary::{
    check_allowlist_miss, ActivationPhase, AllowlistMiss, CanaryConfig, RunTelemetry, Watchdog,
    WatchdogStatus,
};
pub use crate::learner::canary::{ActivationPhase as Phase, CanaryConfig as CanaryOverlay};
pub use crate::learner::compile::FragmentCompiler;
pub use crate::learner::compliance::{
    seed_sources, ComplianceChecker, ComplianceResult, LicenseRecord, SourceEntry,
};
pub use crate::learner::curiosity::CuriosityEngine;
pub use crate::learner::embeddings::EmbeddingsQueue;
pub use crate::learner::eval::{
    check_regression, regression_banner, run_eval, run_injection_probes, EvalMetrics, EvalRun,
    GoldenQuery, RegressionResult, RetrievalResult,
};
pub use crate::learner::governor::{
    degrade_order_description, BudgetConfig, BudgetGovernor, DegradeLevel, EvictionAudit,
    KillSwitchState, SbomReport, TokenBucket,
};
pub use crate::learner::pipeline::AcquisitionPipeline;
pub use crate::learner::report::{AutomatedReport, BudgetSummary, ReportGenerator, ReportPeriod};
pub use crate::learner::scheduler::{LearnerScheduler, SchedulerConfig};
pub use crate::learner::state::{
    compute_sha256, minhash_similarity, ChunkRecord, EmbedPriority, EmbedQueueEntry, PageRecord,
    StateStore, UrlRecord,
};
pub use crate::learner::telemetry::{
    LearnerMetrics, LearnerRunsResponse, PersistentRunLog, RunLogBuffer, RunLogEntry,
    TelemetryAccumulator,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        .map(|p| {
            PathBuf::from(p)
                .join("vortex_atoms_ai")
                .join("learner")
                .join("state.db")
        })
        .unwrap_or_else(|_| PathBuf::from("./learner-state.db"))
}

/// Returns the default directory for learner model downloads.
pub fn learner_models_dir() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|p| {
            PathBuf::from(p)
                .join("vortex_atoms_ai")
                .join("learner")
                .join("models")
        })
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
