use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::atom::{AtomMetrics, AtomState};
use crate::state::{deterministic_embedding, now_epoch_ms, SharedState};
use crate::token_cache::{CognitiveTokenBlock, HotTokenCache, TokenCache};

/// Supported compressed/opaque knowledge-fragment sources.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeFragmentFormat {
    Tcz,
    Bin,
    Unknown,
}

impl KnowledgeFragmentFormat {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("tcz") => Self::Tcz,
            Some("bin") => Self::Bin,
            _ => Self::Unknown,
        }
    }
}

/// Registry entry for a compressed cognitive fragment.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeFragmentDescriptor {
    pub id: String,
    pub intent_label: String,
    pub semantic_hint: String,
    pub path: PathBuf,
    pub format: KnowledgeFragmentFormat,
    pub embedding: Vec<f32>,
    pub estimated_compressed_bytes: usize,
}

impl KnowledgeFragmentDescriptor {
    pub fn new(
        id: impl Into<String>,
        intent_label: impl Into<String>,
        semantic_hint: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        let semantic_hint = semantic_hint.into();
        let path = path.into();
        Self {
            id: id.into(),
            intent_label: intent_label.into(),
            embedding: deterministic_embedding(&semantic_hint),
            semantic_hint,
            format: KnowledgeFragmentFormat::from_path(&path),
            path,
            estimated_compressed_bytes: 0,
        }
    }

    pub fn token_cache_key(&self) -> String {
        format!("{}:{}", self.id, self.path.display())
    }
}

/// Resident compressed fragment bytes loaded on demand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedKnowledgeFragment {
    pub id: String,
    pub intent_label: String,
    pub path: PathBuf,
    pub format: KnowledgeFragmentFormat,
    pub bytes: Arc<[u8]>,
    pub loaded_epoch_ms: u128,
    pub last_access_epoch_ms: u128,
    pub access_count: u64,
}

impl LoadedKnowledgeFragment {
    pub fn new(descriptor: &KnowledgeFragmentDescriptor, bytes: Arc<[u8]>) -> Self {
        let now = now_epoch_ms();
        Self {
            id: descriptor.id.clone(),
            intent_label: descriptor.intent_label.clone(),
            path: descriptor.path.clone(),
            format: descriptor.format,
            bytes,
            loaded_epoch_ms: now,
            last_access_epoch_ms: now,
            access_count: 0,
        }
    }

    pub fn touch(&mut self) {
        self.access_count = self.access_count.saturating_add(1);
        self.last_access_epoch_ms = now_epoch_ms();
    }
}

/// Dynamic action report emitted by the knowledge orchestrator.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum KnowledgeLoadAction {
    TokenCacheHit,
    ResidentFragmentHit,
    DiskLoaded,
    MissingOnDisk,
    UnloadedNonMatchingIntent,
    UnloadedInactive,
}

/// Compact report safe for IKC broadcast events.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct KnowledgeLoadReport {
    pub fragment_id: String,
    pub intent_label: String,
    pub path: String,
    pub action: KnowledgeLoadAction,
    pub bytes: usize,
    pub tokens: usize,
}

/// In-memory orchestration state used by Kernel_02 and Kernel_05.
#[derive(Clone, Debug, PartialEq)]
pub struct KnowledgeOrchestratorState {
    pub registry: HashMap<String, KnowledgeFragmentDescriptor>,
    pub loaded: HashMap<String, LoadedKnowledgeFragment>,
    pub token_cache: HotTokenCache,
    pub max_loaded_fragments: usize,
    /// Stage 7 lifecycle counters (loads, evictions, resident hits/misses).
    pub metrics: AtomMetrics,
}

impl KnowledgeOrchestratorState {
    pub fn new(max_loaded_fragments: usize, max_cached_tokens: usize) -> Self {
        Self {
            registry: HashMap::new(),
            loaded: HashMap::new(),
            token_cache: HotTokenCache::new(max_cached_tokens),
            max_loaded_fragments: max_loaded_fragments.max(1),
            metrics: AtomMetrics::default(),
        }
    }

    /// Atom-model view of one fragment: registered → active/suspended when
    /// resident, evicted when known but unloaded. Pure observer, no mutation.
    pub fn atom_state_of(&self, fragment_id: &str) -> Option<AtomState> {
        if self.loaded.contains_key(fragment_id) {
            Some(AtomState::Active)
        } else if self.registry.contains_key(fragment_id) {
            Some(AtomState::Evicted)
        } else {
            None
        }
    }

    /// Resident atom count (Stage 7 budget input).
    pub fn active_atom_count(&self) -> usize {
        self.loaded.len()
    }

    /// Estimated resident bytes across loaded fragments.
    pub fn resident_bytes(&self) -> usize {
        self.loaded
            .values()
            .map(|fragment| fragment.bytes.len())
            .sum()
    }

    pub fn register(&mut self, descriptor: KnowledgeFragmentDescriptor) {
        self.registry.insert(descriptor.id.clone(), descriptor);
    }

    pub fn register_path(
        &mut self,
        id: impl Into<String>,
        intent_label: impl Into<String>,
        semantic_hint: impl Into<String>,
        path: impl Into<PathBuf>,
    ) {
        self.register(KnowledgeFragmentDescriptor::new(
            id,
            intent_label,
            semantic_hint,
            path,
        ));
    }

    pub fn select_descriptors(
        &self,
        intent_label: &str,
        semantic_text: &str,
        limit: usize,
    ) -> Vec<KnowledgeFragmentDescriptor> {
        let query = deterministic_embedding(semantic_text);
        let mut scored = self
            .registry
            .values()
            .filter(|descriptor| descriptor.intent_label == intent_label)
            .map(|descriptor| {
                let score = cosine_similarity(&query, &descriptor.embedding);
                (score, descriptor.clone())
            })
            .collect::<Vec<_>>();

        scored.sort_by(|left, right| right.0.total_cmp(&left.0));
        scored
            .into_iter()
            .take(limit.max(1))
            .map(|(_, descriptor)| descriptor)
            .collect()
    }

    pub fn touch_loaded_bytes(&mut self, fragment_id: &str) -> Option<Arc<[u8]>> {
        match self.loaded.get_mut(fragment_id) {
            Some(fragment) => {
                fragment.touch();
                self.metrics.record_hit();
                Some(fragment.bytes.clone())
            }
            None => {
                self.metrics.record_miss();
                None
            }
        }
    }

    pub fn insert_loaded(
        &mut self,
        descriptor: &KnowledgeFragmentDescriptor,
        bytes: Arc<[u8]>,
    ) -> Vec<KnowledgeLoadReport> {
        self.loaded.insert(
            descriptor.id.clone(),
            LoadedKnowledgeFragment::new(descriptor, bytes),
        );
        self.metrics.record_load();
        self.enforce_loaded_capacity()
    }

    pub fn unload_non_matching_intent(&mut self, active_intent: &str) -> Vec<KnowledgeLoadReport> {
        let ids_to_remove = self
            .loaded
            .iter()
            .filter(|(_, fragment)| fragment.intent_label.as_str() != active_intent)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();

        ids_to_remove
            .into_iter()
            .filter_map(|id| {
                self.remove_loaded(&id, KnowledgeLoadAction::UnloadedNonMatchingIntent)
            })
            .collect()
    }

    pub fn unload_inactive_older_than(&mut self, older_than_ms: u128) -> Vec<KnowledgeLoadReport> {
        let now = now_epoch_ms();
        let ids_to_remove = self
            .loaded
            .iter()
            .filter_map(|(id, fragment)| {
                let age_ms = now.saturating_sub(fragment.last_access_epoch_ms);
                (age_ms >= older_than_ms).then(|| id.clone())
            })
            .collect::<Vec<_>>();

        ids_to_remove
            .into_iter()
            .filter_map(|id| self.remove_loaded(&id, KnowledgeLoadAction::UnloadedInactive))
            .collect()
    }

    fn enforce_loaded_capacity(&mut self) -> Vec<KnowledgeLoadReport> {
        let mut reports = Vec::new();

        while self.loaded.len() > self.max_loaded_fragments {
            let coldest_id = self
                .loaded
                .iter()
                .min_by_key(|(_, fragment)| (fragment.access_count, fragment.last_access_epoch_ms))
                .map(|(id, _)| id.clone());

            if let Some(id) = coldest_id {
                if let Some(report) = self.remove_loaded(&id, KnowledgeLoadAction::UnloadedInactive)
                {
                    reports.push(report);
                }
            } else {
                break;
            }
        }

        reports
    }

    fn remove_loaded(
        &mut self,
        fragment_id: &str,
        action: KnowledgeLoadAction,
    ) -> Option<KnowledgeLoadReport> {
        let removed = self.loaded.remove(fragment_id)?;
        self.metrics.record_eviction();
        let report = KnowledgeLoadReport {
            fragment_id: removed.id.clone(),
            intent_label: removed.intent_label.clone(),
            path: removed.path.display().to_string(),
            action,
            bytes: removed.bytes.len(),
            tokens: 0,
        };

        // Immediate release point for compressed fragment bytes.
        std::mem::drop(removed);

        Some(report)
    }
}

impl Default for KnowledgeOrchestratorState {
    fn default() -> Self {
        let mut state = Self::new(8, 1_000_000);
        state.register_path(
            "vortex_atoms_ai_logic_core",
            "code_logic",
            "programming code logic execution rust python modules reasoning calculate",
            "knowledge/code_logic_core.bin",
        );
        state.register_path(
            "vortex_atoms_ai_media_core",
            "media",
            "image audio video multimodal render frame generation visual acoustic",
            "knowledge/media_core.tcz",
        );
        state.register_path(
            "vortex_atoms_ai_supervisor_core",
            "supervisor",
            "memory ram purge drop inactive fragments watchdog supervisor budget",
            "knowledge/supervisor_memory.bin",
        );
        state.register_path(
            "engineering_expert_module",
            "code_logic",
            "electronics repair pcb diagnostics lithium battery recycling algorithms metal detector schematics precious metal recovery chemical process safety",
            "knowledge/engineering_expert_module.tcz",
        );
        state.register_path(
            "bio_medical_module",
            "code_logic",
            "human general medicine clinical pharmacology veterinary science avian genetics budgie finch inheritance probability hagoromo blackwing opaline rainbow mutation",
            "knowledge/bio_medical_module.tcz",
        );
        state.register_path(
            "finance_math_module",
            "code_logic",
            "macro micro economics corporate finance factory management workflow logistics risk assessment statistics forecasting monte carlo optimization strategic planning",
            "knowledge/finance_math_module.tcz",
        );
        state.register_path(
            "global_humanities_module",
            "code_logic",
            "grammar translation dictionary world languages global history timeline geography gis mapping coordinates projection spatial index",
            "knowledge/global_humanities_module.bin",
        );
        state
    }
}

/// Orchestrate knowledge based on Kernel_02 semantic intent detection.
///
/// The order is intentionally cache-first:
/// 1. unload resident fragments from unrelated intents;
/// 2. check the hot token cache;
/// 3. use resident compressed bytes if already loaded;
/// 4. read `.tcz`/`.bin` from disk only on a true miss.
pub async fn orchestrate_knowledge_for_intent(
    shared: &SharedState,
    semantic_text: &str,
    intent_label: &str,
) -> Vec<KnowledgeLoadReport> {
    let mut reports = {
        let mut state = shared.write().await;
        state.knowledge.unload_non_matching_intent(intent_label)
    };

    let descriptors = {
        let state = shared.read().await;
        state
            .knowledge
            .select_descriptors(intent_label, semantic_text, 2)
    };

    for descriptor in descriptors {
        let cache_key = descriptor.token_cache_key();

        if let Some(block) = {
            let mut state = shared.write().await;
            state.knowledge.token_cache.get_cached(&cache_key)
        } {
            reports.push(KnowledgeLoadReport {
                fragment_id: descriptor.id.clone(),
                intent_label: descriptor.intent_label.clone(),
                path: descriptor.path.display().to_string(),
                action: KnowledgeLoadAction::TokenCacheHit,
                bytes: block.source_bytes,
                tokens: block.token_count(),
            });
            continue;
        }

        if let Some(bytes) = {
            let mut state = shared.write().await;
            state.knowledge.touch_loaded_bytes(&descriptor.id)
        } {
            let block = CognitiveTokenBlock::from_bytes(descriptor.id.clone(), &bytes);
            let token_count = block.token_count();
            let source_bytes = block.source_bytes;
            {
                let mut state = shared.write().await;
                state.knowledge.token_cache.insert_cached(cache_key, block);
            }
            reports.push(KnowledgeLoadReport {
                fragment_id: descriptor.id.clone(),
                intent_label: descriptor.intent_label.clone(),
                path: descriptor.path.display().to_string(),
                action: KnowledgeLoadAction::ResidentFragmentHit,
                bytes: source_bytes,
                tokens: token_count,
            });
            continue;
        }

        match tokio::fs::read(&descriptor.path).await {
            Ok(bytes) => {
                let bytes: Arc<[u8]> = Arc::from(bytes.into_boxed_slice());
                let block = CognitiveTokenBlock::from_bytes(descriptor.id.clone(), &bytes);
                let token_count = block.token_count();
                let source_bytes = block.source_bytes;

                let mut capacity_reports = {
                    let mut state = shared.write().await;
                    state.knowledge.token_cache.insert_cached(cache_key, block);
                    state.knowledge.insert_loaded(&descriptor, bytes)
                };

                reports.push(KnowledgeLoadReport {
                    fragment_id: descriptor.id.clone(),
                    intent_label: descriptor.intent_label.clone(),
                    path: descriptor.path.display().to_string(),
                    action: KnowledgeLoadAction::DiskLoaded,
                    bytes: source_bytes,
                    tokens: token_count,
                });
                reports.append(&mut capacity_reports);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                reports.push(KnowledgeLoadReport {
                    fragment_id: descriptor.id.clone(),
                    intent_label: descriptor.intent_label.clone(),
                    path: descriptor.path.display().to_string(),
                    action: KnowledgeLoadAction::MissingOnDisk,
                    bytes: 0,
                    tokens: 0,
                });
            }
            Err(_) => {
                reports.push(KnowledgeLoadReport {
                    fragment_id: descriptor.id.clone(),
                    intent_label: descriptor.intent_label.clone(),
                    path: descriptor.path.display().to_string(),
                    action: KnowledgeLoadAction::MissingOnDisk,
                    bytes: 0,
                    tokens: 0,
                });
            }
        }
    }

    reports
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let len = left.len().min(right.len());
    if len == 0 {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut left_norm = 0.0f32;
    let mut right_norm = 0.0f32;

    for index in 0..len {
        dot += left[index] * right[index];
        left_norm += left[index] * left[index];
        right_norm += right[index] * right[index];
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::AtomState;

    fn test_state() -> KnowledgeOrchestratorState {
        let mut state = KnowledgeOrchestratorState::new(8, 1_000);
        state.register_path("frag-a", "code_logic", "rust systems", "knowledge/a.tcz");
        state
    }

    #[test]
    fn atom_observers_track_lifecycle() {
        let mut state = test_state();
        assert_eq!(state.atom_state_of("frag-a"), Some(AtomState::Evicted));
        assert_eq!(state.atom_state_of("unknown"), None);
        assert_eq!(state.active_atom_count(), 0);
        assert_eq!(state.resident_bytes(), 0);

        let descriptor = state.registry.get("frag-a").unwrap().clone();
        let bytes: Arc<[u8]> = Arc::from(vec![1u8; 64].into_boxed_slice());
        state.insert_loaded(&descriptor, bytes);
        assert_eq!(state.atom_state_of("frag-a"), Some(AtomState::Active));
        assert_eq!(state.active_atom_count(), 1);
        assert_eq!(state.resident_bytes(), 64);
        assert_eq!(state.metrics.atom_load_count, 1);
    }

    #[test]
    fn touch_records_hits_and_misses() {
        let mut state = test_state();
        assert!(state.touch_loaded_bytes("frag-a").is_none());
        assert_eq!(state.metrics.atom_cache_misses, 1);

        let descriptor = state.registry.get("frag-a").unwrap().clone();
        let bytes: Arc<[u8]> = Arc::from(vec![2u8; 32].into_boxed_slice());
        state.insert_loaded(&descriptor, bytes);
        assert!(state.touch_loaded_bytes("frag-a").is_some());
        assert_eq!(state.metrics.atom_cache_hits, 1);
    }

    #[test]
    fn eviction_records_metrics() {
        let mut state = KnowledgeOrchestratorState::new(1, 1_000);
        state.register_path("frag-1", "x", "alpha", "knowledge/1.tcz");
        state.register_path("frag-2", "x", "beta", "knowledge/2.tcz");

        let first = state.registry.get("frag-1").unwrap().clone();
        let second = state.registry.get("frag-2").unwrap().clone();
        state.insert_loaded(&first, Arc::from(vec![0u8; 8].into_boxed_slice()));
        state.insert_loaded(&second, Arc::from(vec![0u8; 8].into_boxed_slice()));

        // Capacity 1 forces eviction of the coldest fragment.
        assert_eq!(state.active_atom_count(), 1);
        assert_eq!(state.metrics.atom_load_count, 2);
        assert_eq!(state.metrics.atom_eviction_count, 1);
        assert_eq!(state.atom_state_of("frag-2"), Some(AtomState::Active));
    }
}
