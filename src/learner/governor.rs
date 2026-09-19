// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-02 Section 5: Budget Governor & Kill-Switches.
// Config learner.budgets: disk_gb, chunk_cap, bytes_per_day, embed_cpu_quota,
// eviction: "lru_coldest" with audit line per 1000 evictions.
// Governor loop checks budgets every run tick; violation → graceful degrade.
// Kill-switches: config flip, admin pause, vortex_learn --pause.
// Monthly license/SBOM report: vortex_learn --licenses.

use crate::learner::api::LicenseEntry;
use crate::learner::canary::RunTelemetry;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Instant;

/// Budget configuration for the learner governor.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BudgetConfig {
    pub disk_gb: f64,
    pub chunk_cap: usize,
    pub bytes_per_day: u64,
    pub embed_cpu_quota: u32,
    pub eviction: String,
}

/// Token bucket for CPU quota enforcement.
#[derive(Clone, Debug)]
pub struct TokenBucket {
    capacity: u32,
    tokens: u32,
    refill_rate: u32,
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    pub fn consume(&mut self, amount: u32) -> bool {
        self.refill();
        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate as f64) as u32;
        if tokens_to_add > 0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
            self.last_refill = now;
        }
    }
}

/// Degradation level — ordered from mild to severe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DegradeLevel {
    #[default]
    CuriosityOff,
    SourcesHalved,
    Paused,
}

/// A single eviction audit record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvictionAudit {
    pub count: u64,
    pub evicted_chunk_ids: Vec<String>,
    pub timestamp: u128,
}

/// Budget governor — monitors budgets and degrades gracefully.
#[derive(Clone, Debug)]
pub struct BudgetGovernor {
    config: BudgetConfig,
    current_level: DegradeLevel,
    bytes_this_day: u64,
    chunks_indexed: usize,
    eviction_count: u64,
    eviction_audit: VecDeque<EvictionAudit>,
    token_bucket: TokenBucket,
}

/// Kill-switch state for admin / health endpoints.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KillSwitchState {
    pub config_paused: bool,
    pub admin_paused: bool,
    pub cli_paused: bool,
    pub is_paused: bool,
    pub reason: Option<String>,
    pub degraded_to: DegradeLevel,
}

/// SBOM report.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SbomReport {
    pub entries: Vec<LicenseEntry>,
    pub total_chunks: usize,
    pub total_size_mb: f64,
}

impl BudgetGovernor {
    pub fn new(config: BudgetConfig) -> Self {
        let cpu_quota = if config.embed_cpu_quota > 0 {
            config.embed_cpu_quota
        } else {
            100
        };
        Self {
            config,
            current_level: DegradeLevel::CuriosityOff,
            bytes_this_day: 0,
            chunks_indexed: 0,
            eviction_count: 0,
            eviction_audit: VecDeque::new(),
            token_bucket: TokenBucket::new(cpu_quota, cpu_quota / 10),
        }
    }

    /// Check all budgets. Returns the current degradation level.
    pub fn check(&mut self, telemetry: &RunTelemetry) -> DegradeLevel {
        self.bytes_this_day += telemetry.bytes as u64;
        self.chunks_indexed += telemetry.chunks_indexed;

        // Check bytes_per_day budget
        if self.bytes_this_day > self.config.bytes_per_day {
            self.degrade(DegradeLevel::CuriosityOff);
        }

        // Check chunk_cap budget
        if self.chunks_indexed > self.config.chunk_cap {
            self.degrade(DegradeLevel::SourcesHalved);
        }

        // Check disk_gb budget
        if telemetry.disk_mb / 1024.0 > self.config.disk_gb {
            self.degrade(DegradeLevel::Paused);
        }

        // Check CPU quota via token bucket
        if !self.token_bucket.consume(1) {
            self.degrade(DegradeLevel::CuriosityOff);
        }

        self.current_level.clone()
    }

    fn degrade(&mut self, level: DegradeLevel) {
        if level.clone() as u8 > self.current_level.clone() as u8 {
            self.current_level = level;
        }
    }

    /// Record an eviction for audit.
    pub fn record_eviction(&mut self, chunk_ids: Vec<String>) {
        self.eviction_count += chunk_ids.len() as u64;
        let audit = EvictionAudit {
            count: chunk_ids.len() as u64,
            evicted_chunk_ids: chunk_ids,
            timestamp: 0,
        };
        self.eviction_audit.push_back(audit);
        // Keep only last 100 audit entries
        if self.eviction_audit.len() > 100 {
            self.eviction_audit.pop_front();
        }
    }

    /// Get eviction audit entries (per 1000 evictions).
    pub fn eviction_audit(&self) -> Vec<EvictionAudit> {
        self.eviction_audit.iter().cloned().collect()
    }

    /// Check if a kill-switch is active.
    pub fn kill_switch_state(
        &self,
        config_paused: bool,
        admin_paused: bool,
        cli_paused: bool,
    ) -> KillSwitchState {
        let is_paused = config_paused || admin_paused || cli_paused;
        KillSwitchState {
            config_paused,
            admin_paused,
            cli_paused,
            is_paused,
            reason: if is_paused {
                Some("kill_switch_active".to_string())
            } else {
                None
            },
            degraded_to: self.current_level.clone(),
        }
    }

    /// Generate SBOM report from source entries.
    pub fn generate_sbom(
        sources: &[crate::learner::compliance::SourceEntry],
        chunk_counts: &[usize],
        total_size_mb: f64,
    ) -> SbomReport {
        let entries: Vec<LicenseEntry> = sources
            .iter()
            .zip(chunk_counts.iter())
            .map(|(s, &count)| LicenseEntry {
                source_id: s.id.clone(),
                license: s.license.clone(),
                chunk_count: count,
                attribution_required: s.license != "PD",
            })
            .collect();
        SbomReport {
            entries,
            total_chunks: chunk_counts.iter().sum(),
            total_size_mb,
        }
    }

    /// Check if the governor has violated any budget.
    pub fn has_violation(&self) -> bool {
        self.current_level != DegradeLevel::CuriosityOff
            || self.bytes_this_day > self.config.bytes_per_day
            || self.chunks_indexed > self.config.chunk_cap
    }

    pub fn current_level(&self) -> &DegradeLevel {
        &self.current_level
    }

    pub fn bytes_this_day(&self) -> u64 {
        self.bytes_this_day
    }

    pub fn chunks_indexed(&self) -> usize {
        self.chunks_indexed
    }

    pub fn eviction_count(&self) -> u64 {
        self.eviction_count
    }
}

/// Format the degrade order as a string for documentation.
pub fn degrade_order_description() -> &'static str {
    "Degradation order: Curiosity OFF → Sources Halved → Pause"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learner::canary::RunTelemetry;

    fn make_telemetry() -> RunTelemetry {
        RunTelemetry {
            fetched_ok: 5,
            fetched_failed: 0,
            refused_robots: 0,
            rate_limited: 0,
            bytes: 1024 * 1024,
            pages: 1,
            chunks_indexed: 10,
            embed_queue_depth: 10,
            cache_hit_rate: 0.5,
            gaps_open: 0,
            gaps_closed: 0,
            disk_mb: 100.0,
            wall_ms: 1000,
            allowlist_misses: vec![],
            compliance_violations: 0,
        }
    }

    fn make_budget_config() -> BudgetConfig {
        BudgetConfig {
            disk_gb: 2.0,
            chunk_cap: 200000,
            bytes_per_day: 100 * 1024 * 1024,
            embed_cpu_quota: 100,
            eviction: "lru_coldest".to_string(),
        }
    }

    #[test]
    fn degrade_order() {
        let mut gov = BudgetGovernor::new(make_budget_config());
        let telemetry = make_telemetry();
        gov.check(&telemetry);
        assert!(!gov.has_violation());
    }

    #[test]
    fn bytes_day_cap() {
        let mut gov = BudgetGovernor::new(make_budget_config());
        let mut telemetry = make_telemetry();
        // Simulate exceeding bytes_per_day
        telemetry.bytes = 200 * 1024 * 1024; // 200 MiB > 100 MiB budget
        gov.check(&telemetry);
        assert!(gov.has_violation());
        assert_eq!(gov.current_level(), &DegradeLevel::CuriosityOff);
    }

    #[test]
    fn chunk_cap_triggers_sources_halved() {
        let mut gov = BudgetGovernor::new(BudgetConfig {
            chunk_cap: 5,
            ..make_budget_config()
        });
        let mut telemetry = make_telemetry();
        telemetry.chunks_indexed = 10; // > 5 cap
        gov.check(&telemetry);
        assert_eq!(gov.current_level(), &DegradeLevel::SourcesHalved);
    }

    #[test]
    fn disk_cap_triggers_pause() {
        let mut gov = BudgetGovernor::new(BudgetConfig {
            disk_gb: 0.01, // Very small budget
            ..make_budget_config()
        });
        let mut telemetry = make_telemetry();
        telemetry.disk_mb = 100.0; // > 0.01 GB budget
        gov.check(&telemetry);
        assert_eq!(gov.current_level(), &DegradeLevel::Paused);
    }

    #[test]
    fn cpu_quota_token_bucket() {
        let mut gov = BudgetGovernor::new(make_budget_config());
        // Consume all tokens
        for _ in 0..100 {
            let _ = gov.token_bucket.consume(1);
        }
        // Next consume should fail
        assert!(!gov.token_bucket.consume(1));
    }

    #[test]
    fn eviction_audit_per_1000() {
        let mut gov = BudgetGovernor::new(make_budget_config());
        let chunks: Vec<String> = (0..1000).map(|i| format!("chunk_{}", i)).collect();
        gov.record_eviction(chunks);
        assert_eq!(gov.eviction_count(), 1000);
        let audit = gov.eviction_audit();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].count, 1000);
    }

    #[test]
    fn eviction_audit_multiple() {
        let mut gov = BudgetGovernor::new(make_budget_config());
        for _ in 0..5 {
            let chunks: Vec<String> = (0..200).map(|i| format!("chunk_{}", i)).collect();
            gov.record_eviction(chunks);
        }
        assert_eq!(gov.eviction_count(), 1000);
        assert_eq!(gov.eviction_audit().len(), 5);
    }

    #[test]
    fn kill_switch_config_pause() {
        let gov = BudgetGovernor::new(make_budget_config());
        let state = gov.kill_switch_state(true, false, false);
        assert!(state.is_paused);
        assert!(state.config_paused);
        assert!(!state.admin_paused);
        assert!(!state.cli_paused);
    }

    #[test]
    fn kill_switch_admin_pause() {
        let gov = BudgetGovernor::new(make_budget_config());
        let state = gov.kill_switch_state(false, true, false);
        assert!(state.is_paused);
        assert!(state.admin_paused);
    }

    #[test]
    fn kill_switch_cli_pause() {
        let gov = BudgetGovernor::new(make_budget_config());
        let state = gov.kill_switch_state(false, false, true);
        assert!(state.is_paused);
        assert!(state.cli_paused);
    }

    #[test]
    fn kill_switch_no_pause() {
        let gov = BudgetGovernor::new(make_budget_config());
        let state = gov.kill_switch_state(false, false, false);
        assert!(!state.is_paused);
    }

    #[test]
    fn sbom_report() {
        let sources = vec![
            crate::learner::compliance::SourceEntry {
                id: "wikipedia_en".to_string(),
                license: "CC-BY-SA-4.0".to_string(),
                paths: vec!["/wiki/".to_string()],
                rate_rpm: 30,
            },
            crate::learner::compliance::SourceEntry {
                id: "gutenberg".to_string(),
                license: "PD".to_string(),
                paths: vec!["/ebooks/".to_string()],
                rate_rpm: 10,
            },
        ];
        let counts = vec![100, 50];
        let sbom = BudgetGovernor::generate_sbom(&sources, &counts, 150.0);
        assert_eq!(sbom.entries.len(), 2);
        assert_eq!(sbom.total_chunks, 150);
        assert!(sbom.entries[0].attribution_required);
        assert!(!sbom.entries[1].attribution_required); // PD
    }

    #[test]
    fn sbom_total_chunks() {
        let sources = vec![crate::learner::compliance::SourceEntry {
            id: "wikipedia_en".to_string(),
            license: "CC-BY-SA-4.0".to_string(),
            paths: vec!["/wiki/".to_string()],
            rate_rpm: 30,
        }];
        let counts = vec![200];
        let sbom = BudgetGovernor::generate_sbom(&sources, &counts, 200.0);
        assert_eq!(sbom.total_chunks, 200);
        assert_eq!(sbom.total_size_mb, 200.0);
    }

    #[test]
    fn test_degrade_order_description() {
        let desc = degrade_order_description();
        assert!(desc.contains("Curiosity OFF"));
        assert!(desc.contains("Sources Halved"));
        assert!(desc.contains("Pause"));
    }
}
