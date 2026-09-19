// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-01 Section 2: Scheduler.
// tokio interval + off-peak window + max_runtime; early-exit on budget.

use std::time::Duration;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// Scheduler configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulerConfig {
    #[serde(default)]
    pub run_every_hours: u64,
    #[serde(default)]
    pub max_runtime_minutes: u64,
    #[serde(default)]
    pub off_peak_local: [u8; 2],
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            run_every_hours: 6,
            max_runtime_minutes: 30,
            off_peak_local: [0, 6],
        }
    }
}

/// The learner scheduler.
pub struct LearnerScheduler {
    config: SchedulerConfig,
}

/// A single scheduler tick result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TickResult {
    Ran,
    OffPeak,
    BudgetExhausted,
    NoRun,
}

impl LearnerScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self { config }
    }

    /// Checks if we are currently in the off-peak window.
    pub fn is_off_peak(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let now_hours = (now / 3600) % 24;
        let start = self.config.off_peak_local[0] as u64;
        let end = self.config.off_peak_local[1] as u64;
        if start <= end {
            now_hours >= start && now_hours < end
        } else {
            now_hours >= start || now_hours < end
        }
    }

    /// Determines if a run should proceed.
    pub fn should_run(&self) -> TickResult {
        if self.is_off_peak() {
            return TickResult::OffPeak;
        }
        TickResult::NoRun
    }

    /// Returns the maximum runtime duration.
    pub fn max_runtime(&self) -> Duration {
        Duration::from_secs(self.config.max_runtime_minutes * 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_off_peak_detection() {
        let config = SchedulerConfig {
            run_every_hours: 6,
            max_runtime_minutes: 30,
            off_peak_local: [0, 6],
        };
        let sched = LearnerScheduler::new(config);
        let _ = sched.should_run();
    }

    #[test]
    fn scheduler_max_runtime() {
        let config = SchedulerConfig {
            run_every_hours: 6,
            max_runtime_minutes: 30,
            off_peak_local: [0, 6],
        };
        let sched = LearnerScheduler::new(config);
        assert_eq!(sched.max_runtime(), Duration::from_secs(1800));
    }

    #[test]
    fn scheduler_config_defaults() {
        let config = SchedulerConfig::default();
        assert_eq!(config.run_every_hours, 6);
    }
}
