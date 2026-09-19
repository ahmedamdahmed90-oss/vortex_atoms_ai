use crate::ikc::{IkcEvent, IkcMessage, KernelCommand, KernelId, MediaType};
use crate::knowledge_orchestrator::orchestrate_knowledge_for_intent;
use crate::state::{deterministic_embedding, SharedState};
use tokio::sync::{broadcast, mpsc};

/// Kernel_02: Router & Vector DB.
///
/// Handles semantic intent routing and coordinates with the local in-memory
/// Qdrant-style vector client stored in shared state.
pub async fn run(
    mut inbox: mpsc::Receiver<IkcMessage>,
    code_tx: mpsc::Sender<IkcMessage>,
    media_tx: mpsc::Sender<IkcMessage>,
    supervisor_tx: mpsc::Sender<IkcMessage>,
    events: broadcast::Sender<IkcEvent>,
    shared: SharedState,
) {
    let kernel = KernelId::Kernel02RouterVectorDb;
    let _ = events.send(IkcEvent::Started { kernel });

    while let Some(message) = inbox.recv().await {
        match message.command {
            KernelCommand::RouteIntent { request_id, text } => {
                let route = classify_intent(&text, &shared).await;
                let (target, routed_command, intent) = match route {
                    RouteDecision::CodeLogic => (
                        KernelId::Kernel03CodeLogicExpert,
                        KernelCommand::ExecuteLogic {
                            request_id: request_id.clone(),
                            module: select_logic_module(&text),
                            payload: text.clone(),
                        },
                        "code_logic".to_string(),
                    ),
                    RouteDecision::LlmGeneration => (
                        KernelId::Kernel03CodeLogicExpert,
                        KernelCommand::LlmGenerate {
                            request_id: request_id.clone(),
                            prompt: text.clone(),
                            max_tokens: None,
                            temperature: None,
                            top_p: None,
                            stream: true,
                        },
                        "llm_generation".to_string(),
                    ),
                    RouteDecision::Media(media_type) => (
                        KernelId::Kernel04MultimodalMedia,
                        KernelCommand::RenderMedia {
                            request_id: request_id.clone(),
                            media_type,
                            prompt: text.clone(),
                            frames: frame_budget(&text),
                        },
                        "media".to_string(),
                    ),
                    RouteDecision::Supervisor => (
                        KernelId::Kernel05SupervisorWatchdog,
                        KernelCommand::PurgeInactiveFragments { older_than_ms: 0 },
                        "supervisor".to_string(),
                    ),
                };

                let knowledge_reports =
                    orchestrate_knowledge_for_intent(&shared, &text, &intent).await;

                {
                    let mut state = shared.write().await;
                    state.router.routed_requests = state.router.routed_requests.saturating_add(1);
                    state.router.last_intent = Some(intent.clone());
                    state.router.vector_db.upsert(
                        request_id.clone(),
                        deterministic_embedding(&text),
                        intent.clone(),
                    );

                    // Force the local index through qdrant-client's PointStruct
                    // boundary so Kernel_02 remains compatible with Qdrant while
                    // still serving the hot path from in-process memory.
                    let _qdrant_compatible_points = state.router.vector_db.qdrant_points_snapshot();
                }

                if !knowledge_reports.is_empty() {
                    let _ = events.send(IkcEvent::KnowledgeOrchestrated {
                        kernel,
                        request_id: request_id.clone(),
                        reports: knowledge_reports,
                    });
                }

                let _ = events.send(IkcEvent::Routed {
                    kernel,
                    request_id: request_id.clone(),
                    target,
                    intent,
                });

                let send_result = match target {
                    KernelId::Kernel03CodeLogicExpert => {
                        code_tx
                            .send(IkcMessage::new(kernel, target, routed_command))
                            .await
                    }
                    KernelId::Kernel04MultimodalMedia => {
                        media_tx
                            .send(IkcMessage::new(kernel, target, routed_command))
                            .await
                    }
                    KernelId::Kernel05SupervisorWatchdog => {
                        supervisor_tx
                            .send(IkcMessage::new(kernel, target, routed_command))
                            .await
                    }
                    _ => {
                        let _ = events.send(IkcEvent::Warning {
                            kernel,
                            message: format!(
                                "Kernel_02: unexpected target {target:?} for request {request_id}"
                            ),
                        });
                        Ok(())
                    }
                };

                if let Err(e) = send_result {
                    let _ = events.send(IkcEvent::Warning {
                        kernel,
                        message: format!("Kernel_02 failed to dispatch request {request_id}: {e}"),
                    });
                }
            }
            KernelCommand::RegisterKnowledgeFragment { descriptor } => {
                let descriptor_id = descriptor.id.clone();
                let intent_label = descriptor.intent_label.clone();
                let semantic_hint = descriptor.semantic_hint.clone();
                let embedding = descriptor.embedding.clone();

                {
                    let mut state = shared.write().await;
                    state.knowledge.register(descriptor);
                    state.router.vector_db.upsert(
                        format!("knowledge:{descriptor_id}"),
                        embedding,
                        intent_label.clone(),
                    );
                    let _qdrant_compatible_points = state.router.vector_db.qdrant_points_snapshot();
                }

                let _ = events.send(IkcEvent::Warning {
                    kernel,
                    message: format!(
                        "Kernel_02 registered knowledge fragment '{descriptor_id}' for intent '{intent_label}' with hint '{semantic_hint}'"
                    ),
                });
            }
            KernelCommand::Shutdown => {
                let _ = events.send(IkcEvent::ShutdownRequested { kernel });
                break;
            }
            unsupported => {
                let _ = events.send(IkcEvent::Warning {
                    kernel,
                    message: format!(
                        "Kernel_02 rejected unsupported command for Router & Vector DB: {unsupported:?}"
                    ),
                });
            }
        }

        tokio::task::yield_now().await;
    }

    let _ = events.send(IkcEvent::Stopped { kernel });
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RouteDecision {
    CodeLogic,
    LlmGeneration,
    Media(MediaType),
    Supervisor,
}

async fn classify_intent(text: &str, shared: &SharedState) -> RouteDecision {
    let lower = text.to_ascii_lowercase();

    if contains_any(
        &lower,
        &[
            "purge",
            "drop memory",
            "free ram",
            "watchdog",
            "supervisor",
            "inactive fragments",
        ],
    ) {
        return RouteDecision::Supervisor;
    }

    if contains_any(
        &lower,
        &[
            "image",
            "audio",
            "video",
            "render",
            "frame",
            "multimodal",
            "media",
            "draw",
        ],
    ) {
        return RouteDecision::Media(select_media_type(&lower));
    }

    if contains_any(
        &lower,
        &[
            "write",
            "compose",
            "essay",
            "story",
            "poem",
            "explain",
            "describe",
            "tell me",
            "generate text",
            "chat",
            "conversation",
            "answer",
            "question",
            "what is",
            "how to",
            "why is",
            "who is",
            "help me",
            "think",
            "reason",
            "analyze this",
            "summarize",
            "translate",
            "rewrite",
            "creative",
            "brainstorm",
        ],
    ) {
        return RouteDecision::LlmGeneration;
    }

    if contains_any(
        &lower,
        &[
            "code",
            "logic",
            "program",
            "rust",
            "python",
            "function",
            "module",
            "execute",
            "run",
            "calculate",
            "sum",
            "electronics",
            "repair",
            "pcb",
            "circuit board",
            "lithium",
            "battery recycling",
            "metal detector",
            "schematic",
            "precious metal",
            "gold extraction",
            "chemical process",
            "diagnostics",
            "medicine",
            "clinical",
            "pharmacology",
            "veterinary",
            "avian",
            "bird",
            "budgie",
            "finch",
            "genetics",
            "inheritance",
            "mutation",
            "hagoromo",
            "blackwing",
            "opaline",
            "rainbow",
            "finance",
            "economics",
            "macroeconomic",
            "microeconomic",
            "corporate",
            "factory",
            "workflow",
            "logistics",
            "risk assessment",
            "statistics",
            "forecasting",
            "monte carlo",
            "strategic planning",
            "npv",
            "wacc",
            "grammar",
            "translation",
            "dictionary",
            "language",
            "multilingual",
            "world history",
            "timeline",
            "geography",
            "gis",
            "mapping",
            "coordinate",
            "projection",
            "spatial",
            "geohash",
            "haversine",
            "r-tree",
        ],
    ) {
        return RouteDecision::CodeLogic;
    }

    let embedding = deterministic_embedding(text);
    let best_payload = {
        let state = shared.read().await;
        state
            .router
            .vector_db
            .search_best(&embedding)
            .map(|(_, payload, score)| (payload.to_string(), score))
    };

    match best_payload {
        Some((payload, score)) if payload == "media" && score > 0.05 => {
            RouteDecision::Media(select_media_type(&lower))
        }
        Some((payload, score)) if payload == "supervisor" && score > 0.05 => {
            RouteDecision::Supervisor
        }
        _ => RouteDecision::CodeLogic,
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

/// Deterministic intent gate that routes avian-genetics crosses to the embedded
/// fastpath kernel. Requires both an avian species/species-entry keyword *and* a
/// genetics/inheritance or cross signal so unrelated questions (e.g. "bird
/// watching", "finch nesting") never hit the fastpath.
fn is_avian_genetics_intent(lower: &str) -> bool {
    let avian = contains_any(
        lower,
        &[
            "avian",
            "bird",
            "budgie",
            "budgerigar",
            "finch",
            "zebra finch",
            "cockatiel",
            "lovebird",
            "parakeet",
            "hagoromo",
            "hagoromo",
        ],
    );
    if !avian {
        return false;
    }
    contains_any(
        lower,
        &[
            "genetics",
            "inheritance",
            "inherit",
            "cross",
            "x ",
            "×",
            "pairing",
            "mutation",
            "opaline",
            "opline",
            "rainbow",
            "blackwing",
            "double factor",
            "single factor",
            "sex-linked",
            "phenotype",
            "offspring",
        ],
    )
}

fn select_media_type(text: &str) -> MediaType {
    if text.contains("video") {
        MediaType::Video
    } else if text.contains("audio") {
        MediaType::Audio
    } else if text.contains("image") || text.contains("draw") {
        MediaType::Image
    } else {
        MediaType::Multimodal
    }
}

fn select_logic_module(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "grammar",
            "syntax",
            "morphology",
            "translation",
            "translate",
            "dictionary",
            "language",
            "multilingual",
            "mandarin",
            "arabic",
            "spanish",
            "hindi",
            "world history",
            "history timeline",
            "geography",
            "gis",
            "mapping",
            "coordinate",
            "projection",
            "spatial",
            "geohash",
            "haversine",
            "r-tree",
        ],
    ) {
        "global.humanities".to_string()
    } else if contains_any(
        &lower,
        &[
            "finance",
            "financial",
            "economics",
            "macroeconomic",
            "microeconomic",
            "corporate",
            "factory",
            "workflow",
            "logistics",
            "risk assessment",
            "risk framework",
            "statistics",
            "statistical",
            "forecasting",
            "monte carlo",
            "strategic planning",
            "npv",
            "irr",
            "wacc",
            "supply chain",
            "throughput",
            "oee",
        ],
    ) {
        "finance.math".to_string()
    } else if is_avian_genetics_intent(&lower) {
        "bio.avian.fastpath".to_string()
    } else if contains_any(
        &lower,
        &[
            "pharmacology",
            "drug",
            "patient",
            "veterinary",
            "avian",
            "bird",
            "budgie",
            "finch",
            "genetics",
            "inheritance",
            "mutation",
            "hagoromo",
            "blackwing",
            "opaline",
            "rainbow",
        ],
    ) {
        "bio.medical".to_string()
    } else if contains_any(
        &lower,
        &[
            "electronics",
            "repair",
            "pcb",
            "circuit board",
            "lithium",
            "battery recycling",
            "metal detector",
            "schematic",
            "precious metal",
            "gold extraction",
            "chemical process",
            "diagnostics",
        ],
    ) {
        "engineering.expert".to_string()
    } else if lower.contains("sum") || lower.contains("calculate") || lower.contains("math") {
        "math.sum".to_string()
    } else if lower.contains("code") || lower.contains("program") || lower.contains("rust") {
        "code.analysis".to_string()
    } else {
        "logic.reasoning".to_string()
    }
}

fn frame_budget(text: &str) -> usize {
    let lower = text.to_ascii_lowercase();
    if lower.contains("video") {
        24
    } else if lower.contains("audio") {
        8
    } else {
        1
    }
}
