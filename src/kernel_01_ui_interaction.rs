use crate::ikc::{IkcEvent, IkcMessage, KernelCommand, KernelId};
use crate::state::{KnowledgeFragment, SharedState};
use tokio::sync::{broadcast, mpsc};

/// Kernel_01: UI & Interaction.
///
/// Dedicated to user-facing input capture and interaction state. It accepts UI
/// messages, stores lightweight interaction fragments, and forwards intent
/// routing work to Kernel_02 over IKC.
pub async fn run(
    mut inbox: mpsc::Receiver<IkcMessage>,
    router_tx: mpsc::Sender<IkcMessage>,
    events: broadcast::Sender<IkcEvent>,
    shared: SharedState,
) {
    let kernel = KernelId::Kernel01UiInteraction;
    let _ = events.send(IkcEvent::Started { kernel });

    while let Some(message) = inbox.recv().await {
        match message.command {
            KernelCommand::UiInput { session_id, text } => {
                let request_id = next_request_id("ui");

                {
                    let mut state = shared.write().await;
                    state.ui.active_session_id = Some(session_id.clone());
                    state.ui.last_input = Some(text.clone());
                    state.ui.interaction_count = state.ui.interaction_count.saturating_add(1);
                    state
                        .knowledge_fragments
                        .push_back(KnowledgeFragment::inactive(
                            request_id.clone(),
                            "ui_input".to_string(),
                            text.clone(),
                        ));
                }

                let _ = events.send(IkcEvent::UiAccepted {
                    kernel,
                    session_id,
                    request_id: request_id.clone(),
                });

                let route_message = IkcMessage::new(
                    kernel,
                    KernelId::Kernel02RouterVectorDb,
                    KernelCommand::RouteIntent { request_id, text },
                );

                if router_tx.send(route_message).await.is_err() {
                    let _ = events.send(IkcEvent::Warning {
                        kernel,
                        message: "Kernel_01 could not forward input to Kernel_02".to_string(),
                    });
                }
            }
            KernelCommand::Shutdown => {
                let _ = events.send(IkcEvent::ShutdownRequested { kernel });
                break;
            }
            unsupported => {
                let _ = events.send(IkcEvent::Warning {
                    kernel,
                    message: format!(
                        "Kernel_01 rejected unsupported command for UI & Interaction: {unsupported:?}"
                    ),
                });
            }
        }

        tokio::task::yield_now().await;
    }

    let _ = events.send(IkcEvent::Stopped { kernel });
}

fn next_request_id(prefix: &str) -> String {
    format!(
        "vortex_atoms_ai_{prefix}_{}_{}",
        std::process::id(),
        crate::state::now_epoch_ms()
    )
}
