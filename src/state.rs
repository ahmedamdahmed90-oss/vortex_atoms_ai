use qdrant_client::{qdrant::PointStruct, Payload};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use crate::knowledge_orchestrator::KnowledgeOrchestratorState;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Shared state handle protected by Arc<RwLock<T>> for safe cross-kernel access.
pub type SharedState = Arc<RwLock<VortexAtomsSharedState>>;

/// Current Unix epoch in milliseconds. Used for retention and purge decisions.
pub fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

/// Knowledge fragments are intentionally explicit so Kernel_05 can drop them.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeFragment {
    pub id: String,
    pub label: String,
    pub bytes_estimate: usize,
    pub active: bool,
    pub last_access_epoch_ms: u128,
    pub content: String,
}

impl KnowledgeFragment {
    pub fn inactive(id: String, label: String, content: String) -> Self {
        Self {
            id,
            label,
            bytes_estimate: content.len(),
            active: false,
            last_access_epoch_ms: now_epoch_ms(),
            content,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct UiState {
    pub active_session_id: Option<String>,
    pub last_input: Option<String>,
    pub interaction_count: u64,
}

/// Lightweight in-memory vector database facade used by Kernel_02.
///
/// It intentionally mirrors the role Qdrant plays in production routing while
/// staying local and deterministic for low-latency tests. The Cargo dependency
/// `qdrant-client` remains available for replacing this facade with a networked
/// Qdrant deployment when desired.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InMemoryQdrantClient {
    pub collection_name: String,
    vectors: HashMap<String, Vec<f32>>,
    payloads: HashMap<String, String>,
}

impl InMemoryQdrantClient {
    pub fn new(collection_name: impl Into<String>) -> Self {
        Self {
            collection_name: collection_name.into(),
            vectors: HashMap::new(),
            payloads: HashMap::new(),
        }
    }

    pub fn upsert(&mut self, id: impl Into<String>, vector: Vec<f32>, payload: impl Into<String>) {
        let id = id.into();
        self.vectors.insert(id.clone(), vector);
        self.payloads.insert(id, payload.into());
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn search_best(&self, query: &[f32]) -> Option<(&str, &str, f32)> {
        self.vectors
            .iter()
            .filter_map(|(id, vector)| {
                self.payloads.get(id).map(|payload| {
                    let score = cosine_similarity(query, vector);
                    (id.as_str(), payload.as_str(), score)
                })
            })
            .max_by(|left, right| left.2.total_cmp(&right.2))
    }

    /// Export the in-memory index through qdrant-client point structures.
    ///
    /// Kernel_02 keeps search local for low latency, but the boundary type is
    /// `qdrant_client::qdrant::PointStruct`, making this index wire-compatible
    /// with a real Qdrant collection when one is attached later.
    pub fn qdrant_points_snapshot(&self) -> Vec<PointStruct> {
        self.vectors
            .iter()
            .map(|(id, vector)| PointStruct::new(id.clone(), vector.clone(), Payload::default()))
            .collect()
    }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RouterState {
    pub routed_requests: u64,
    pub last_intent: Option<String>,
    pub vector_db: InMemoryQdrantClient,
}

impl Default for RouterState {
    fn default() -> Self {
        let mut vector_db = InMemoryQdrantClient::new("vortex_atoms_ai_intents");
        vector_db.upsert(
            "intent_code_logic",
            deterministic_embedding("code logic programming rust module execute"),
            "code_logic",
        );
        vector_db.upsert(
            "intent_media",
            deterministic_embedding("image audio video media render frame multimodal"),
            "media",
        );
        vector_db.upsert(
            "intent_supervisor",
            deterministic_embedding("memory purge drop inactive watchdog supervisor ram"),
            "supervisor",
        );
        vector_db.upsert(
            "intent_finance_math",
            deterministic_embedding("macro micro economics corporate finance factory logistics risk statistics forecasting strategic planning"),
            "code_logic",
        );
        vector_db.upsert(
            "intent_global_humanities",
            deterministic_embedding("grammar translation dictionary world languages history timeline geography gis mapping coordinates projection spatial"),
            "code_logic",
        );

        Self {
            routed_requests: 0,
            last_intent: None,
            vector_db,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CodeLogicState {
    pub modules_executed: u64,
    pub last_module: Option<String>,
    pub last_result_summary: Option<String>,
    pub last_llm_result: Option<String>,
    pub llm_generations_completed: u64,
    pub llm_total_tokens_generated: u64,
    pub llm_model_loaded: bool,
    pub last_token_rate: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct MediaState {
    pub frames_rendered: u64,
    pub last_media_type: Option<String>,
    pub last_prompt: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SupervisorState {
    pub purge_cycles: u64,
    pub dropped_fragments: u64,
    pub memory_budget_bytes: usize,
}

impl Default for SupervisorState {
    fn default() -> Self {
        Self {
            purge_cycles: 0,
            dropped_fragments: 0,
            memory_budget_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Complete 5-Kernel Matrix shared memory surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VortexAtomsSharedState {
    pub ui: UiState,
    pub router: RouterState,
    pub code_logic: CodeLogicState,
    pub media: MediaState,
    pub supervisor: SupervisorState,
    pub knowledge_fragments: VecDeque<KnowledgeFragment>,
    pub knowledge: KnowledgeOrchestratorState,
}

pub fn new_shared_state(memory_budget_bytes: usize) -> SharedState {
    let mut state = VortexAtomsSharedState::default();
    state.supervisor.memory_budget_bytes = memory_budget_bytes;
    Arc::new(RwLock::new(state))
}

/// Deterministic 32-dimensional lexical embedding for local semantic routing.
pub fn deterministic_embedding(text: &str) -> Vec<f32> {
    const DIMENSIONS: usize = 32;
    let mut vector = vec![0.0f32; DIMENSIONS];

    for token in text.split_whitespace() {
        let mut hash = 2_166_136_261u32;
        for byte in token.bytes() {
            hash ^= u32::from(byte);
            hash = hash.wrapping_mul(16_777_619);
        }
        let index = (hash as usize) % DIMENSIONS;
        vector[index] += 1.0;
    }

    vector
}
