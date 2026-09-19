// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-02 Section 6: Report Automation.
// Generates automated monthly/quarterly reports from learner telemetry.
// Reports are JSON and markdown; persisted to learner_reports/ directory.

use crate::learner::compliance::SourceEntry;
use crate::learner::eval::EvalMetrics;
use crate::learner::governor::{
    BudgetConfig, BudgetGovernor, DegradeLevel, KillSwitchState, SbomReport,
};
use crate::learner::telemetry::{LearnerMetrics, PersistentRunLog};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Automated report generator for learner runs.
#[derive(Clone, Debug)]
pub struct ReportGenerator {
    report_dir: PathBuf,
    budget: BudgetConfig,
}

/// A single automated report.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AutomatedReport {
    pub report_id: String,
    pub generated_at: u128,
    pub period: ReportPeriod,
    pub budget_summary: BudgetSummary,
    pub kill_switch_state: KillSwitchState,
    pub sbom: SbomReport,
    pub eval_metrics: Option<EvalMetrics>,
    pub mrr_trend: Vec<(String, f64)>,
    pub total_runs: usize,
    pub total_chunks_indexed: usize,
    pub total_bytes: u64,
    pub total_evictions: u64,
}

/// The reporting period.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ReportPeriod {
    #[default]
    Monthly,
    Quarterly,
    Annual,
}

/// Budget summary for a reporting period.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BudgetSummary {
    pub disk_gb: f64,
    pub chunk_cap: usize,
    pub bytes_per_day: u64,
    pub embed_cpu_quota: u32,
    pub eviction_policy: String,
    pub avg_degrade_level: DegradeLevel,
    pub total_evictions: u64,
}

impl ReportGenerator {
    /// Create a new report generator targeting `report_dir`.
    pub fn new(report_dir: PathBuf, budget: BudgetConfig) -> Self {
        Self { report_dir, budget }
    }

    /// Generate a monthly automated report from the run log and metrics.
    pub fn generate_monthly_report(
        &self,
        _run_log: &PersistentRunLog,
        metrics: &LearnerMetrics,
        sources: &[SourceEntry],
        eval_metrics: Option<EvalMetrics>,
        mrr_trend: Vec<(String, f64)>,
    ) -> AutomatedReport {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let total_runs = metrics.total_runs;
        let total_chunks_indexed = metrics.total_chunks_indexed;
        let total_bytes = metrics.total_bytes as u64;
        let total_evictions = metrics.total_evictions;

        let avg_degrade = DegradeLevel::CuriosityOff;

        let sbom = BudgetGovernor::generate_sbom(
            sources,
            &[total_chunks_indexed],
            metrics.current_disk_mb / 1024.0,
        );

        let kill_switch =
            BudgetGovernor::new(self.budget.clone()).kill_switch_state(false, false, false);

        let budget_summary = BudgetSummary {
            disk_gb: self.budget.disk_gb,
            chunk_cap: self.budget.chunk_cap,
            bytes_per_day: self.budget.bytes_per_day,
            embed_cpu_quota: self.budget.embed_cpu_quota,
            eviction_policy: self.budget.eviction.clone(),
            avg_degrade_level: avg_degrade,
            total_evictions,
        };

        let report = AutomatedReport {
            report_id: format!("monthly_{}", now),
            generated_at: now,
            period: ReportPeriod::Monthly,
            budget_summary,
            kill_switch_state: kill_switch,
            sbom,
            eval_metrics,
            mrr_trend,
            total_runs,
            total_chunks_indexed,
            total_bytes,
            total_evictions,
        };

        Self::persist_report(&self.report_dir, &report);
        report
    }

    /// Generate a quarterly report by aggregating monthly reports.
    pub fn generate_quarterly_report(
        &self,
        monthly_reports: Vec<AutomatedReport>,
        sources: &[SourceEntry],
    ) -> AutomatedReport {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let total_runs: usize = monthly_reports.iter().map(|r| r.total_runs).sum();
        let total_chunks: usize = monthly_reports.iter().map(|r| r.total_chunks_indexed).sum();
        let total_bytes: u64 = monthly_reports.iter().map(|r| r.total_bytes).sum();
        let total_evictions: u64 = monthly_reports.iter().map(|r| r.total_evictions).sum();
        let mrr_trend: Vec<(String, f64)> = monthly_reports
            .iter()
            .flat_map(|r| r.mrr_trend.clone())
            .collect();

        let eval_metrics = monthly_reports.iter().find_map(|r| r.eval_metrics.clone());

        let sbom = BudgetGovernor::generate_sbom(
            sources,
            &[total_chunks],
            total_bytes as f64 / 1024.0 / 1024.0,
        );

        let kill_switch =
            BudgetGovernor::new(self.budget.clone()).kill_switch_state(false, false, false);

        let budget_summary = BudgetSummary {
            disk_gb: self.budget.disk_gb,
            chunk_cap: self.budget.chunk_cap,
            bytes_per_day: self.budget.bytes_per_day,
            embed_cpu_quota: self.budget.embed_cpu_quota,
            eviction_policy: self.budget.eviction.clone(),
            avg_degrade_level: DegradeLevel::CuriosityOff,
            total_evictions,
        };

        let report = AutomatedReport {
            report_id: format!("quarterly_{}", now),
            generated_at: now,
            period: ReportPeriod::Quarterly,
            budget_summary,
            kill_switch_state: kill_switch,
            sbom,
            eval_metrics,
            mrr_trend,
            total_runs,
            total_chunks_indexed: total_chunks,
            total_bytes,
            total_evictions,
        };

        Self::persist_report(&self.report_dir, &report);
        report
    }

    fn persist_report(report_dir: &Path, report: &AutomatedReport) {
        let json_path = report_dir.join(format!("{}.json", report.report_id));
        let md_path = report_dir.join(format!("{}.md", report.report_id));

        if let Ok(json) = serde_json::to_string_pretty(report) {
            let _ = Self::write_file(&json_path, &json);
        }

        let md = Self::render_markdown(report);
        let _ = Self::write_file(&md_path, &md);
    }

    fn render_markdown(report: &AutomatedReport) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Automated Report: {}\n\n", report.report_id));
        md.push_str(&format!("**Generated:** {}\n\n", report.generated_at));
        md.push_str(&format!("**Period:** {:?}\n\n", report.period));

        md.push_str("## Budget Summary\n\n");
        md.push_str(&format!(
            "- Disk budget: {:.1} GB\n",
            report.budget_summary.disk_gb
        ));
        md.push_str(&format!(
            "- Chunk cap: {}\n",
            report.budget_summary.chunk_cap
        ));
        md.push_str(&format!(
            "- Bytes/day: {}\n",
            report.budget_summary.bytes_per_day
        ));
        md.push_str(&format!(
            "- CPU quota: {}\n",
            report.budget_summary.embed_cpu_quota
        ));
        md.push_str(&format!(
            "- Eviction policy: {}\n",
            report.budget_summary.eviction_policy
        ));
        md.push_str(&format!(
            "- Total evictions: {}\n\n",
            report.budget_summary.total_evictions
        ));

        md.push_str("## Kill Switch State\n\n");
        md.push_str(&format!(
            "- Config paused: {}\n",
            report.kill_switch_state.config_paused
        ));
        md.push_str(&format!(
            "- Admin paused: {}\n",
            report.kill_switch_state.admin_paused
        ));
        md.push_str(&format!(
            "- CLI paused: {}\n",
            report.kill_switch_state.cli_paused
        ));
        md.push_str(&format!(
            "- Is paused: {}\n\n",
            report.kill_switch_state.is_paused
        ));

        md.push_str("## SBOM\n\n");
        md.push_str(&format!("- Total sources: {}\n", report.sbom.entries.len()));
        md.push_str(&format!("- Total chunks: {}\n\n", report.sbom.total_chunks));
        for entry in &report.sbom.entries {
            md.push_str(&format!(
                "- {} ({}) — chunks: {}, attribution: {}\n",
                entry.source_id, entry.license, entry.chunk_count, entry.attribution_required
            ));
        }

        md.push_str("\n## Metrics\n\n");
        md.push_str(&format!("- Total runs: {}\n", report.total_runs));
        md.push_str(&format!(
            "- Total chunks indexed: {}\n",
            report.total_chunks_indexed
        ));
        md.push_str(&format!("- Total bytes: {}\n", report.total_bytes));
        md.push_str(&format!(
            "- Total evictions: {}\n\n",
            report.total_evictions
        ));

        if let Some(eval) = &report.eval_metrics {
            md.push_str("## Evaluation\n\n");
            md.push_str(&format!("- MRR@5: {:.4}\n", eval.mrr_at_5));
            md.push_str(&format!("- Hit@1: {:.4}\n\n", eval.hit_at_1));
        }

        md
    }

    fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
        let parent = path.parent();
        if let Some(p) = parent {
            std::fs::create_dir_all(p).ok();
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Get the report directory.
    pub fn report_dir(&self) -> &PathBuf {
        &self.report_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_report_generator() -> ReportGenerator {
        ReportGenerator::new(
            PathBuf::from("./target/test_reports"),
            BudgetConfig::default(),
        )
    }

    fn make_run_log() -> PersistentRunLog {
        PersistentRunLog::new(PathBuf::from("./target/test_runs.jsonl")).unwrap()
    }

    fn make_metrics() -> LearnerMetrics {
        LearnerMetrics {
            total_runs: 5,
            total_chunks_indexed: 1000,
            total_bytes: 104857600,
            total_evictions: 10,
            current_disk_mb: 2048.0,
            ..Default::default()
        }
    }

    fn make_sources() -> Vec<SourceEntry> {
        vec![crate::learner::compliance::SourceEntry {
            id: "wikipedia_en".to_string(),
            license: "CC-BY-SA-4.0".to_string(),
            paths: vec!["/wiki/".to_string()],
            rate_rpm: 30,
        }]
    }

    fn make_eval_metrics() -> EvalMetrics {
        EvalMetrics {
            mrr_at_5: 0.63,
            hit_at_1: 0.75,
            ..Default::default()
        }
    }

    #[test]
    fn monthly_report_generation() {
        let gen = make_report_generator();
        let _run_log = make_run_log();
        let metrics = make_metrics();
        let sources = make_sources();
        let eval = Some(make_eval_metrics());
        let mrr_trend = vec![("2026-01".to_string(), 0.50), ("2026-02".to_string(), 0.63)];

        let report = gen.generate_monthly_report(&_run_log, &metrics, &sources, eval, mrr_trend);

        assert_eq!(report.period, ReportPeriod::Monthly);
        assert_eq!(report.total_runs, 5);
        assert_eq!(report.total_chunks_indexed, 1000);
        assert!(report.total_bytes > 0);
        assert_eq!(report.total_evictions, 10);
        assert_eq!(report.sbom.entries.len(), 1);
        assert!(!report.report_id.is_empty());
    }

    #[test]
    fn quarterly_report_generation() {
        let gen = make_report_generator();
        let sources = make_sources();

        let monthly_reports = vec![
            AutomatedReport {
                report_id: "monthly_1".to_string(),
                generated_at: 0,
                period: ReportPeriod::Monthly,
                budget_summary: BudgetSummary {
                    disk_gb: 2.0,
                    chunk_cap: 200000,
                    bytes_per_day: 104857600,
                    embed_cpu_quota: 100,
                    eviction_policy: "lru_coldest".to_string(),
                    avg_degrade_level: DegradeLevel::CuriosityOff,
                    total_evictions: 3,
                },
                kill_switch_state: KillSwitchState::default(),
                sbom: SbomReport {
                    entries: vec![],
                    total_chunks: 300,
                    total_size_mb: 100.0,
                },
                eval_metrics: Some(make_eval_metrics()),
                mrr_trend: vec![("2026-01".to_string(), 0.50)],
                total_runs: 2,
                total_chunks_indexed: 300,
                total_bytes: 30000000,
                total_evictions: 3,
            },
            AutomatedReport {
                report_id: "monthly_2".to_string(),
                generated_at: 0,
                period: ReportPeriod::Monthly,
                budget_summary: BudgetSummary {
                    disk_gb: 2.0,
                    chunk_cap: 200000,
                    bytes_per_day: 104857600,
                    embed_cpu_quota: 100,
                    eviction_policy: "lru_coldest".to_string(),
                    avg_degrade_level: DegradeLevel::CuriosityOff,
                    total_evictions: 4,
                },
                kill_switch_state: KillSwitchState::default(),
                sbom: SbomReport {
                    entries: vec![],
                    total_chunks: 400,
                    total_size_mb: 200.0,
                },
                eval_metrics: None,
                mrr_trend: vec![("2026-02".to_string(), 0.63)],
                total_runs: 2,
                total_chunks_indexed: 400,
                total_bytes: 40000000,
                total_evictions: 4,
            },
        ];

        let report = gen.generate_quarterly_report(monthly_reports, &sources);

        assert_eq!(report.period, ReportPeriod::Quarterly);
        assert_eq!(report.total_runs, 4);
        assert_eq!(report.total_chunks_indexed, 700);
        assert_eq!(report.total_evictions, 7);
    }

    #[test]
    fn markdown_rendering() {
        let gen = make_report_generator();
        let _run_log = make_run_log();
        let metrics = make_metrics();
        let sources = make_sources();
        let report = gen.generate_monthly_report(&_run_log, &metrics, &sources, None, vec![]);

        let md = ReportGenerator::render_markdown(&report);
        assert!(md.contains("Automated Report"));
        assert!(md.contains("Budget Summary"));
        assert!(md.contains("SBOM"));
        assert!(md.contains("Kill Switch State"));
    }

    #[test]
    fn report_persistence() {
        let dir = std::env::temp_dir();
        let gen = ReportGenerator::new(dir, BudgetConfig::default());
        let _run_log = make_run_log();
        let metrics = make_metrics();
        let sources = make_sources();
        let report = gen.generate_monthly_report(&_run_log, &metrics, &sources, None, vec![]);

        let json_path = std::env::temp_dir().join(format!("{}.json", report.report_id));
        assert!(json_path.exists());

        let md_path = std::env::temp_dir().join(format!("{}.md", report.report_id));
        assert!(md_path.exists());
    }
}
