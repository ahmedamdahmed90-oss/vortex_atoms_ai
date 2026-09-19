use crate::ikc::{IkcEvent, IkcMessage, KernelCommand, KernelId, MediaType};
use crate::state::{KnowledgeFragment, SharedState};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, Duration};

/// Kernel_04: Multimodal & Media.
///
/// Dedicated to rendering and frame-by-frame generation of media. It emits a
/// broadcast event for every generated frame/segment.
pub async fn run(
    mut inbox: mpsc::Receiver<IkcMessage>,
    events: broadcast::Sender<IkcEvent>,
    shared: SharedState,
) {
    let kernel = KernelId::Kernel04MultimodalMedia;
    let _ = events.send(IkcEvent::Started { kernel });

    while let Some(message) = inbox.recv().await {
        match message.command {
            KernelCommand::RenderMedia {
                request_id,
                media_type,
                prompt,
                frames,
            } => {
                let total_frames = frames.max(1);
                for frame_index in 0..total_frames {
                    {
                        let mut state = shared.write().await;
                        state.media.frames_rendered = state.media.frames_rendered.saturating_add(1);
                        state.media.last_media_type =
                            Some(media_type_label(&media_type).to_string());
                        state.media.last_prompt = Some(prompt.clone());
                    }

                    let _ = events.send(IkcEvent::MediaFrameRendered {
                        kernel,
                        request_id: request_id.clone(),
                        media_type: media_type.clone(),
                        frame_index,
                        total_frames,
                    });

                    sleep(Duration::from_millis(render_tick_ms(&media_type))).await;
                }

                {
                    let mut state = shared.write().await;
                    state
                        .knowledge_fragments
                        .push_back(KnowledgeFragment::inactive(
                            request_id,
                            format!("media_render:{}", media_type_label(&media_type)),
                            format!(
                                "rendered {total_frames} frame(s) for {} prompt: {prompt}",
                                media_type_label(&media_type)
                            ),
                        ));
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
                        "Kernel_04 rejected unsupported command for Multimodal & Media: {unsupported:?}"
                    ),
                });
            }
        }

        tokio::task::yield_now().await;
    }

    let _ = events.send(IkcEvent::Stopped { kernel });
}

fn render_tick_ms(media_type: &MediaType) -> u64 {
    match media_type {
        MediaType::Image => 8,
        MediaType::Audio => 12,
        MediaType::Video => 16,
        MediaType::Multimodal => 20,
    }
}

fn media_type_label(media_type: &MediaType) -> &'static str {
    match media_type {
        MediaType::Image => "image",
        MediaType::Audio => "audio",
        MediaType::Video => "video",
        MediaType::Multimodal => "multimodal",
    }
}
