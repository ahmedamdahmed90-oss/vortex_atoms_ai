//! Vortex Atoms AI kernel foundation.
//!
//! This crate provides the low-latency machine-learning kernel structure for
//! `vortex_atoms_ai`. The central design goal is to keep large model weight
//! files outside of heap memory by mapping them read-only with `memmap2`, then
//! materializing only the tensor slices required by the runtime.
//!
//! The crate also exposes the Vortex Atoms AI 5-Kernel Matrix: five independent
//! Tokio tasks linked through asynchronous mpsc and broadcast IKC channels, with
//! shared state protected by `Arc<RwLock<T>>`.

pub mod bio_medical_module;
pub mod build_info;
#[cfg(feature = "dashboard")]
pub mod dashboard;
pub mod engineering_expert_module;
pub mod error;
pub mod fastpath;
pub mod finance_math_module;
pub mod five_kernel_matrix;
pub mod global_humanities_module;
pub mod ikc;
pub mod inference;
pub mod kernel;
pub mod kernel_01_ui_interaction;
pub mod kernel_02_router_vector_db;
pub mod kernel_03_code_logic_expert;
pub mod kernel_04_multimodal_media;
pub mod kernel_05_supervisor_watchdog;
pub mod knowledge_orchestrator;
#[cfg(feature = "learner")]
pub mod learner;
pub mod llm_api;
pub mod llm_cache;
pub mod llm_config;
pub mod llm_download;
pub mod llm_embed;
pub mod llm_inference;
pub mod llm_model;
pub mod llm_prompt;
pub mod llm_sampling;
pub mod llm_stream;
pub mod llm_tools;
pub mod model_ladder;

pub mod embedded_frontend;
pub mod knowledge_import;
pub mod llm_agent;
pub mod llm_mcp;
pub mod llm_memory;
pub mod llm_neural_embed;
pub mod llm_plugin;
pub mod llm_structured;
pub mod llm_ws;
pub mod mmap_weights;
pub mod perf_topology;
pub mod security;
pub mod session_store;
pub mod speculative;
pub mod state;
pub mod system_tray;
pub mod token_cache;
pub mod vortex_config;

pub use crate::bio_medical_module::{
    bio_medical_descriptor, run_bio_medical_module, BioMedicalAnswer, BioMedicalKnowledgeSection,
    BioMedicalModule, BIO_MEDICAL_MAX_MAPPED_BYTES, BIO_MEDICAL_MODULE_ID, BIO_MEDICAL_MODULE_PATH,
};
pub use crate::engineering_expert_module::{
    engineering_expert_descriptor, run_engineering_expert_module, EngineeringExpertAnswer,
    EngineeringExpertModule, EngineeringKnowledgeSection, ENGINEERING_EXPERT_MODULE_ID,
    ENGINEERING_EXPERT_MODULE_PATH,
};
pub use crate::error::{Result, VortexAtomsError};
pub use crate::fastpath::{
    run_avian_genetics_fastpath, AvianFastpathManifest, AvianGeneticsFastpath, FastpathAnswer,
    FastpathMutation, FastpathTemplates, OutcomeTemplate, ParentPhenotype, FASTPATH_MODULE_ID,
    FASTPATH_SCHEMA, FASTPATH_SUMMARY_MARKER, FASTPATH_VERSION,
};
pub use crate::finance_math_module::{
    finance_math_descriptor, run_finance_math_module, FinanceKnowledgeSection, FinanceMathAnswer,
    FinanceMathModule, FINANCE_MATH_MAX_MAPPED_BYTES, FINANCE_MATH_MODULE_ID,
    FINANCE_MATH_MODULE_PATH,
};
pub use crate::five_kernel_matrix::{FiveKernelMatrix, FiveKernelMatrixConfig, KernelIngress};
pub use crate::global_humanities_module::{
    global_humanities_descriptor, run_global_humanities_module, GlobalHumanitiesAnswer,
    GlobalHumanitiesModule, GlobalHumanitiesSection, GLOBAL_HUMANITIES_MAX_MAPPED_BYTES,
    GLOBAL_HUMANITIES_MODULE_ID, GLOBAL_HUMANITIES_MODULE_PATH,
};
pub use crate::ikc::{IkcEvent, IkcMessage, KernelCommand, KernelId, MediaType};
pub use crate::kernel::{DevicePreference, KernelConfig, VortexAtomsKernel};
pub use crate::knowledge_orchestrator::{
    KnowledgeFragmentDescriptor, KnowledgeFragmentFormat, KnowledgeLoadAction, KnowledgeLoadReport,
    KnowledgeOrchestratorState,
};
pub use crate::mmap_weights::{MmapWeights, TensorSpec, WeightDType};
pub use crate::state::{SharedState, VortexAtomsSharedState};
pub use crate::token_cache::{CognitiveTokenBlock, HotTokenCache, TokenCache};
