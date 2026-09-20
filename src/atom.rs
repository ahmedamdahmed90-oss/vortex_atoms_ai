// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//! Stage 6/7 — Atom abstraction + centralized resource budget.
//!
//! An **Atom** is the unit of loadable runtime capability: a knowledge
//! fragment, a fastpath module, or an inference session. The model is
//! deliberately thin: identity + kind + lifecycle state + budget estimate.
//! The existing [`KnowledgeOrchestratorState`](crate::knowledge_orchestrator::KnowledgeOrchestratorState)
//! already implements registry → load → touch → evict; this module gives
//! that lifecycle a shared vocabulary, a tested state machine, centralized
//! budget constants, and observable metrics — without changing behavior.

use serde::{Deserialize, Serialize};

/// What kind of capability an atom carries.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AtomKind {
    KnowledgeFragment,
    FastpathModule,
    InferenceSession,
}

/// Lifecycle states. Legal transitions are enforced by
/// [`AtomState::can_transition`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AtomState {
    Registered,
    Inactive,
    Loading,
    Active,
    Suspended,
    Evicted,
    Failed,
}

impl AtomState {
    /// Returns true when `self -> next` is a legal lifecycle step.
    ///
    /// The machine is intentionally strict: resurrection goes through
    /// `Evicted -> Loading`, and terminal failure is sticky until an
    /// explicit re-register (`Failed -> Registered`).
    pub fn can_transition(self, next: Self) -> bool {
        use AtomState::*;
        matches!(
            (self, next),
            (Registered, Loading)
                | (Registered, Inactive)
                | (Inactive, Loading)
                | (Loading, Active)
                | (Loading, Failed)
                | (Active, Suspended)
                | (Active, Evicted)
                | (Active, Failed)
                | (Suspended, Active)
                | (Suspended, Evicted)
                | (Evicted, Loading)
                | (Failed, Registered)
        )
    }

    pub fn is_resident(self) -> bool {
        matches!(self, Self::Active | Self::Suspended)
    }
}

/// Static identity + budget estimate for one atom.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtomDescriptor {
    pub id: String,
    pub kind: AtomKind,
    /// Estimated resident bytes when active (heuristic, not a promise).
    pub estimated_bytes: usize,
    pub state: AtomState,
}

impl AtomDescriptor {
    pub fn new(id: impl Into<String>, kind: AtomKind, estimated_bytes: usize) -> Self {
        Self {
            id: id.into(),
            kind,
            estimated_bytes,
            state: AtomState::Registered,
        }
    }

    /// Attempt a lifecycle step; returns false (no state change) when illegal.
    pub fn transition(&mut self, next: AtomState) -> bool {
        if self.state.can_transition(next) {
            self.state = next;
            true
        } else {
            false
        }
    }
}

/// Centralized resource budget. Defaults mirror the previously hard-coded
/// constants so introducing this type changes no behavior:
///
/// - `max_memory_bytes`: 5-Kernel matrix + shared-state budget (256 MiB).
/// - `max_active_atoms`: orchestrator `max_loaded_fragments` default (8).
/// - `max_concurrent_inference`: API inference semaphore (2).
/// - `max_context_tokens`: static upper bound for budgeting; per-tier model
///   limits still apply at generation time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ResourceBudget {
    pub max_memory_bytes: usize,
    pub max_active_atoms: usize,
    pub max_concurrent_inference: usize,
    pub max_context_tokens: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024,
            max_active_atoms: 8,
            max_concurrent_inference: 2,
            max_context_tokens: 4096,
        }
    }
}

/// Observable lifecycle counters (Stage 7 metrics).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct AtomMetrics {
    pub atom_load_count: u64,
    pub atom_eviction_count: u64,
    pub atom_cache_hits: u64,
    pub atom_cache_misses: u64,
    pub atom_active_duration_ms_total: u128,
}

impl AtomMetrics {
    pub fn record_load(&mut self) {
        self.atom_load_count = self.atom_load_count.saturating_add(1);
    }

    pub fn record_eviction(&mut self) {
        self.atom_eviction_count = self.atom_eviction_count.saturating_add(1);
    }

    pub fn record_hit(&mut self) {
        self.atom_cache_hits = self.atom_cache_hits.saturating_add(1);
    }

    pub fn record_miss(&mut self) {
        self.atom_cache_misses = self.atom_cache_misses.saturating_add(1);
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.atom_cache_hits + self.atom_cache_misses;
        if total == 0 {
            0.0
        } else {
            self.atom_cache_hits as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lifecycle_is_legal() {
        let mut atom = AtomDescriptor::new("frag-1", AtomKind::KnowledgeFragment, 1024);
        assert_eq!(atom.state, AtomState::Registered);
        for next in [
            AtomState::Loading,
            AtomState::Active,
            AtomState::Suspended,
            AtomState::Active,
            AtomState::Evicted,
            AtomState::Loading,
            AtomState::Active,
        ] {
            assert!(atom.transition(next), "transition to {next:?} rejected");
        }
        assert!(atom.state.is_resident());
    }

    #[test]
    fn illegal_transitions_rejected() {
        let mut atom = AtomDescriptor::new("frag-2", AtomKind::KnowledgeFragment, 0);
        // Registered -> Active skips Loading.
        assert!(!atom.transition(AtomState::Active));
        assert_eq!(atom.state, AtomState::Registered);
        // Evicted -> Active skips Loading.
        assert!(atom.transition(AtomState::Inactive));
        assert!(atom.transition(AtomState::Loading));
        assert!(atom.transition(AtomState::Failed));
        assert!(!atom.transition(AtomState::Active));
        // Failure is sticky until re-register.
        assert!(atom.transition(AtomState::Registered));
    }

    #[test]
    fn suspend_evict_paths() {
        let mut atom = AtomDescriptor::new("frag-3", AtomKind::FastpathModule, 512);
        assert!(atom.transition(AtomState::Loading));
        assert!(atom.transition(AtomState::Active));
        assert!(atom.transition(AtomState::Suspended));
        assert!(atom.state.is_resident());
        assert!(atom.transition(AtomState::Evicted));
        assert!(!atom.state.is_resident());
    }

    #[test]
    fn budget_defaults_match_legacy_constants() {
        let budget = ResourceBudget::default();
        assert_eq!(budget.max_memory_bytes, 256 * 1024 * 1024);
        assert_eq!(budget.max_active_atoms, 8);
        assert_eq!(budget.max_concurrent_inference, 2);
        assert_eq!(budget.max_context_tokens, 4096);
    }

    #[test]
    fn metrics_hit_rate() {
        let mut metrics = AtomMetrics::default();
        assert_eq!(metrics.hit_rate(), 0.0);
        metrics.record_hit();
        metrics.record_hit();
        metrics.record_miss();
        metrics.record_load();
        metrics.record_eviction();
        assert!((metrics.hit_rate() - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(metrics.atom_load_count, 1);
        assert_eq!(metrics.atom_eviction_count, 1);
    }
}
