use serde::{Deserialize, Serialize};

use crate::knowledge_orchestrator::{KnowledgeFragmentDescriptor, KnowledgeLoadReport};

/// Stable identifiers for the Vortex Atoms AI 5-Kernel Matrix.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KernelId {
    Kernel01UiInteraction,
    Kernel02RouterVectorDb,
    Kernel03CodeLogicExpert,
    Kernel04MultimodalMedia,
    Kernel05SupervisorWatchdog,
}

impl KernelId {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Kernel01UiInteraction => "Kernel_01",
            Self::Kernel02RouterVectorDb => "Kernel_02",
            Self::Kernel03CodeLogicExpert => "Kernel_03",
            Self::Kernel04MultimodalMedia => "Kernel_04",
            Self::Kernel05SupervisorWatchdog => "Kernel_05",
        }
    }

    pub const fn responsibility(self) -> &'static str {
        match self {
            Self::Kernel01UiInteraction => "UI & Interaction",
            Self::Kernel02RouterVectorDb => "Router & Vector DB",
            Self::Kernel03CodeLogicExpert => "Code & Logic Expert",
            Self::Kernel04MultimodalMedia => "Multimodal & Media",
            Self::Kernel05SupervisorWatchdog => "Supervisor & Watchdog",
        }
    }
}

/// Commands exchanged over Inter-Kernel Communication (IKC) mpsc channels.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "command")]
pub enum KernelCommand {
    /// User-facing input accepted by Kernel_01.
    UiInput { session_id: String, text: String },
    /// Semantic intent route request processed by Kernel_02.
    RouteIntent { request_id: String, text: String },
    /// Programming/logic module execution request for Kernel_03 only.
    ExecuteLogic {
        request_id: String,
        module: String,
        payload: String,
    },
    /// Media rendering request for Kernel_04 only.
    RenderMedia {
        request_id: String,
        media_type: MediaType,
        prompt: String,
        frames: usize,
    },
    /// Dynamically register a compressed `.tcz`/`.bin` knowledge fragment.
    RegisterKnowledgeFragment {
        descriptor: KnowledgeFragmentDescriptor,
    },
    /// Strict memory purge request for Kernel_05.
    PurgeInactiveFragments { older_than_ms: u128 },
    /// Graceful task shutdown.
    Shutdown,

    /// LLM text generation request routed to Kernel_03.
    LlmGenerate {
        request_id: String,
        prompt: String,
        max_tokens: Option<usize>,
        temperature: Option<f64>,
        top_p: Option<f64>,
        stream: bool,
    },

    /// Cancel an ongoing LLM generation.
    LlmCancel { request_id: String },
}

/// Supported media domains for Kernel_04.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Audio,
    Video,
    Multimodal,
}

/// mpsc envelope for directed IKC messages.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IkcMessage {
    pub source: KernelId,
    pub target: KernelId,
    pub command: KernelCommand,
}

impl IkcMessage {
    pub fn new(source: KernelId, target: KernelId, command: KernelCommand) -> Self {
        Self {
            source,
            target,
            command,
        }
    }
}

/// broadcast event stream used for observability and fan-out notifications.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum IkcEvent {
    Started {
        kernel: KernelId,
    },
    Stopped {
        kernel: KernelId,
    },
    UiAccepted {
        kernel: KernelId,
        session_id: String,
        request_id: String,
    },
    Routed {
        kernel: KernelId,
        request_id: String,
        target: KernelId,
        intent: String,
    },
    LogicExecuted {
        kernel: KernelId,
        request_id: String,
        module: String,
        summary: String,
    },
    MediaFrameRendered {
        kernel: KernelId,
        request_id: String,
        media_type: MediaType,
        frame_index: usize,
        total_frames: usize,
    },
    KnowledgeOrchestrated {
        kernel: KernelId,
        request_id: String,
        reports: Vec<KnowledgeLoadReport>,
    },
    MemoryPurged {
        kernel: KernelId,
        dropped_fragments: usize,
        remaining_fragments: usize,
    },
    Warning {
        kernel: KernelId,
        message: String,
    },
    ShutdownRequested {
        kernel: KernelId,
    },

    /// Single token generated during LLM inference.
    LlmTokenGenerated {
        kernel: KernelId,
        request_id: String,
        token_id: u32,
        token_text: String,
        position: usize,
    },

    /// Full LLM generation completed.
    LlmGenerationComplete {
        kernel: KernelId,
        request_id: String,
        total_tokens: usize,
        tokens_per_second: f64,
        full_text: String,
    },

    /// LLM error occurred.
    LlmError {
        kernel: KernelId,
        request_id: String,
        error_message: String,
    },
}
