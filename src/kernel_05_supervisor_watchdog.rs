use crate::ikc::{IkcEvent, IkcMessage, KernelCommand, KernelId};
use crate::state::{now_epoch_ms, KnowledgeFragment, SharedState};
use std::collections::VecDeque;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval, Duration};

/// Kernel_05: Supervisor & Watchdog.
///
/// Strict memory manager for Vortex Atoms AI. It periodically purges inactive
/// knowledge fragments and explicitly calls `std::mem::drop` on removed
/// fragments so their heap allocations are released as soon as possible.
pub async fn run(
    mut inbox: mpsc::Receiver<IkcMessage>,
    events: broadcast::Sender<IkcEvent>,
    shared: SharedState,
    default_retention_ms: u128,
) {
    let kernel = KernelId::Kernel05SupervisorWatchdog;
    let _ = events.send(IkcEvent::Started { kernel });

    let mut watchdog_tick = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            maybe_message = inbox.recv() => {
                match maybe_message {
                    Some(message) => {
                        match message.command {
                            KernelCommand::PurgeInactiveFragments { older_than_ms } => {
                                let (dropped_fragments, remaining_fragments) = purge_inactive_fragments(
                                    &shared,
                                    older_than_ms,
                                ).await;
                                let _ = events.send(IkcEvent::MemoryPurged {
                                    kernel,
                                    dropped_fragments,
                                    remaining_fragments,
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
                                        "Kernel_05 rejected unsupported command for Supervisor & Watchdog: {unsupported:?}"
                                    ),
                                });
                            }
                        }
                    }
                    None => break,
                }
            }
            _ = watchdog_tick.tick() => {
                let (dropped_fragments, remaining_fragments) = purge_inactive_fragments(
                    &shared,
                    default_retention_ms,
                ).await;
                let _ = events.send(IkcEvent::MemoryPurged {
                    kernel,
                    dropped_fragments,
                    remaining_fragments,
                });
            }
        }

        tokio::task::yield_now().await;
    }

    let _ = events.send(IkcEvent::Stopped { kernel });
}

async fn purge_inactive_fragments(shared: &SharedState, older_than_ms: u128) -> (usize, usize) {
    let now = now_epoch_ms();
    let mut fragments_to_drop = Vec::<KnowledgeFragment>::new();
    let remaining_fragments;
    let extra_drops;

    {
        let mut state = shared.write().await;
        let mut retained = VecDeque::with_capacity(state.knowledge_fragments.len());

        while let Some(fragment) = state.knowledge_fragments.pop_front() {
            let age_ms = now.saturating_sub(fragment.last_access_epoch_ms);
            if !fragment.active && age_ms >= older_than_ms {
                fragments_to_drop.push(fragment);
            } else {
                retained.push_back(fragment);
            }
        }

        state.knowledge_fragments = retained;

        let unloaded_knowledge = state.knowledge.unload_inactive_older_than(older_than_ms);
        let evicted_token_blocks = state
            .knowledge
            .token_cache
            .evict_inactive_older_than(older_than_ms);
        extra_drops = unloaded_knowledge
            .len()
            .saturating_add(evicted_token_blocks);
        std::mem::drop(unloaded_knowledge);

        remaining_fragments = state
            .knowledge_fragments
            .len()
            .saturating_add(state.knowledge.loaded.len())
            .saturating_add(state.knowledge.token_cache.len());
        state.supervisor.purge_cycles = state.supervisor.purge_cycles.saturating_add(1);
        state.supervisor.dropped_fragments = state
            .supervisor
            .dropped_fragments
            .saturating_add(fragments_to_drop.len().saturating_add(extra_drops) as u64);
    }

    let dropped_fragments = fragments_to_drop.len().saturating_add(extra_drops);

    // Explicit strict purge point requested by the 5-Kernel Matrix design.
    // This drops the removed fragments immediately instead of letting the vector
    // live until the surrounding async function returns.
    std::mem::drop(fragments_to_drop);

    (dropped_fragments, remaining_fragments)
}
