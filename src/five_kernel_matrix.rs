use std::sync::Arc;

use crate::ikc::{IkcEvent, IkcMessage, KernelCommand, KernelId};
use crate::kernel_01_ui_interaction;
use crate::kernel_02_router_vector_db;
use crate::kernel_03_code_logic_expert;
use crate::kernel_04_multimodal_media;
use crate::kernel_05_supervisor_watchdog;
use crate::knowledge_orchestrator::KnowledgeFragmentDescriptor;
use crate::llm_config::LlmConfig;
use crate::llm_inference::LlmInference;
use crate::state::{new_shared_state, SharedState};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::JoinHandle;

/// Configuration for the asynchronous Vortex Atoms AI 5-Kernel Matrix.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FiveKernelMatrixConfig {
    pub ikc_channel_capacity: usize,
    pub broadcast_channel_capacity: usize,
    pub memory_budget_bytes: usize,
    pub supervisor_default_retention_ms: u128,
    pub llm_config: Option<LlmConfig>,
}

impl Default for FiveKernelMatrixConfig {
    fn default() -> Self {
        Self {
            ikc_channel_capacity: 256,
            broadcast_channel_capacity: 512,
            memory_budget_bytes: crate::atom::ResourceBudget::default().max_memory_bytes,
            supervisor_default_retention_ms: 60_000,
            llm_config: None,
        }
    }
}

/// mpsc ingress channels for direct IKC dispatch to each kernel.
#[derive(Clone)]
pub struct KernelIngress {
    pub kernel_01: mpsc::Sender<IkcMessage>,
    pub kernel_02: mpsc::Sender<IkcMessage>,
    pub kernel_03: mpsc::Sender<IkcMessage>,
    pub kernel_04: mpsc::Sender<IkcMessage>,
    pub kernel_05: mpsc::Sender<IkcMessage>,
}

/// Running 5-Kernel Matrix handle.
pub struct FiveKernelMatrix {
    ingress: KernelIngress,
    events: broadcast::Sender<IkcEvent>,
    shared: SharedState,
    handles: Vec<JoinHandle<()>>,
}

impl FiveKernelMatrix {
    /// Spawn Kernel_01 through Kernel_05 as independent Tokio tasks.
    pub fn spawn(config: FiveKernelMatrixConfig) -> Self {
        let capacity = config.ikc_channel_capacity.max(1);

        let (kernel_01_tx, kernel_01_rx) = mpsc::channel(capacity);
        let (kernel_02_tx, kernel_02_rx) = mpsc::channel(capacity);
        let (kernel_03_tx, kernel_03_rx) = mpsc::channel(capacity);
        let (kernel_04_tx, kernel_04_rx) = mpsc::channel(capacity);
        let (kernel_05_tx, kernel_05_rx) = mpsc::channel(capacity);

        let (events, _) = broadcast::channel(config.broadcast_channel_capacity.max(1));
        let shared = new_shared_state(config.memory_budget_bytes);

        let llm_engine: Option<Arc<Mutex<LlmInference>>> =
            config
                .llm_config
                .and_then(|cfg| match LlmInference::load(&cfg) {
                    Ok(engine) => {
                        println!(
                            "[VortexKernel] LLM model loaded successfully from {}",
                            cfg.model_path.display()
                        );
                        Some(Arc::new(Mutex::new(engine)))
                    }
                    Err(e) => {
                        eprintln!("[VortexKernel] Failed to load LLM model: {e}");
                        None
                    }
                });

        let handles = vec![
            tokio::spawn(kernel_01_ui_interaction::run(
                kernel_01_rx,
                kernel_02_tx.clone(),
                events.clone(),
                shared.clone(),
            )),
            tokio::spawn(kernel_02_router_vector_db::run(
                kernel_02_rx,
                kernel_03_tx.clone(),
                kernel_04_tx.clone(),
                kernel_05_tx.clone(),
                events.clone(),
                shared.clone(),
            )),
            tokio::spawn(kernel_03_code_logic_expert::run(
                kernel_03_rx,
                events.clone(),
                shared.clone(),
                llm_engine,
            )),
            tokio::spawn(kernel_04_multimodal_media::run(
                kernel_04_rx,
                events.clone(),
                shared.clone(),
            )),
            tokio::spawn(kernel_05_supervisor_watchdog::run(
                kernel_05_rx,
                events.clone(),
                shared.clone(),
                config.supervisor_default_retention_ms,
            )),
        ];

        Self {
            ingress: KernelIngress {
                kernel_01: kernel_01_tx,
                kernel_02: kernel_02_tx,
                kernel_03: kernel_03_tx,
                kernel_04: kernel_04_tx,
                kernel_05: kernel_05_tx,
            },
            events,
            shared,
            handles,
        }
    }

    pub fn ingress(&self) -> KernelIngress {
        self.ingress.clone()
    }

    pub fn shared_state(&self) -> SharedState {
        self.shared.clone()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<IkcEvent> {
        self.events.subscribe()
    }

    /// Submit a user input into Kernel_01.
    pub async fn submit_ui_input(
        &self,
        session_id: impl Into<String>,
        text: impl Into<String>,
    ) -> std::result::Result<(), mpsc::error::SendError<IkcMessage>> {
        self.ingress
            .kernel_01
            .send(IkcMessage::new(
                KernelId::Kernel01UiInteraction,
                KernelId::Kernel01UiInteraction,
                KernelCommand::UiInput {
                    session_id: session_id.into(),
                    text: text.into(),
                },
            ))
            .await
    }

    /// Ask Kernel_05 to immediately purge inactive fragments older than the
    /// supplied age threshold.
    pub async fn request_purge(
        &self,
        older_than_ms: u128,
    ) -> std::result::Result<(), mpsc::error::SendError<IkcMessage>> {
        self.ingress
            .kernel_05
            .send(IkcMessage::new(
                KernelId::Kernel05SupervisorWatchdog,
                KernelId::Kernel05SupervisorWatchdog,
                KernelCommand::PurgeInactiveFragments { older_than_ms },
            ))
            .await
    }

    /// Register a compressed `.tcz` or `.bin` knowledge fragment with Kernel_02
    /// so future semantic intent detection can load it on demand.
    pub async fn register_knowledge_fragment(
        &self,
        descriptor: KnowledgeFragmentDescriptor,
    ) -> std::result::Result<(), mpsc::error::SendError<IkcMessage>> {
        self.ingress
            .kernel_02
            .send(IkcMessage::new(
                KernelId::Kernel02RouterVectorDb,
                KernelId::Kernel02RouterVectorDb,
                KernelCommand::RegisterKnowledgeFragment { descriptor },
            ))
            .await
    }

    /// Send a directed IKC message to one kernel.
    pub async fn send_to_kernel(
        &self,
        message: IkcMessage,
    ) -> std::result::Result<(), mpsc::error::SendError<IkcMessage>> {
        match message.target {
            KernelId::Kernel01UiInteraction => self.ingress.kernel_01.send(message).await,
            KernelId::Kernel02RouterVectorDb => self.ingress.kernel_02.send(message).await,
            KernelId::Kernel03CodeLogicExpert => self.ingress.kernel_03.send(message).await,
            KernelId::Kernel04MultimodalMedia => self.ingress.kernel_04.send(message).await,
            KernelId::Kernel05SupervisorWatchdog => self.ingress.kernel_05.send(message).await,
        }
    }

    /// Gracefully stop all five Tokio tasks and await their termination.
    pub async fn shutdown(mut self) {
        let shutdown_targets = [
            (
                KernelId::Kernel01UiInteraction,
                self.ingress.kernel_01.clone(),
            ),
            (
                KernelId::Kernel02RouterVectorDb,
                self.ingress.kernel_02.clone(),
            ),
            (
                KernelId::Kernel03CodeLogicExpert,
                self.ingress.kernel_03.clone(),
            ),
            (
                KernelId::Kernel04MultimodalMedia,
                self.ingress.kernel_04.clone(),
            ),
            (
                KernelId::Kernel05SupervisorWatchdog,
                self.ingress.kernel_05.clone(),
            ),
        ];

        for (target, tx) in shutdown_targets {
            let _ = tx
                .send(IkcMessage::new(target, target, KernelCommand::Shutdown))
                .await;
        }

        while let Some(handle) = self.handles.pop() {
            let _ = handle.await;
        }
    }
}
