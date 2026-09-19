use std::sync::Arc;

use crate::bio_medical_module::run_bio_medical_module;
use crate::engineering_expert_module::run_engineering_expert_module;
use crate::fastpath::run_avian_genetics_fastpath;
use crate::finance_math_module::run_finance_math_module;
use crate::global_humanities_module::run_global_humanities_module;
use crate::ikc::{IkcEvent, IkcMessage, KernelCommand, KernelId};
use crate::llm_inference::LlmInference;
use crate::llm_stream::StreamEvent;
use crate::state::{KnowledgeFragment, SharedState};
use tokio::sync::{broadcast, mpsc, Mutex};

pub async fn run(
    mut inbox: mpsc::Receiver<IkcMessage>,
    events: broadcast::Sender<IkcEvent>,
    shared: SharedState,
    llm_engine: Option<Arc<Mutex<LlmInference>>>,
) {
    let kernel = KernelId::Kernel03CodeLogicExpert;
    let _ = events.send(IkcEvent::Started { kernel });

    while let Some(message) = inbox.recv().await {
        match message.command {
            KernelCommand::ExecuteLogic {
                request_id,
                module,
                payload,
            } => {
                let module_for_task = module.clone();
                let payload_for_task = payload.clone();
                let summary = match tokio::task::spawn_blocking(move || {
                    execute_logic_module(&module_for_task, &payload_for_task)
                })
                .await
                {
                    Ok(summary) => summary,
                    Err(join_error) => format!("logic module join failure: {join_error}"),
                };

                {
                    let mut state = shared.write().await;
                    state.code_logic.modules_executed =
                        state.code_logic.modules_executed.saturating_add(1);
                    state.code_logic.last_module = Some(module.clone());
                    state.code_logic.last_result_summary = Some(summary.clone());
                    state
                        .knowledge_fragments
                        .push_back(KnowledgeFragment::inactive(
                            request_id.clone(),
                            format!("logic_result:{module}"),
                            summary.clone(),
                        ));
                }

                let _ = events.send(IkcEvent::LogicExecuted {
                    kernel,
                    request_id,
                    module,
                    summary,
                });
            }

            KernelCommand::LlmGenerate {
                request_id,
                prompt,
                max_tokens,
                temperature,
                top_p: _,
                stream,
            } => {
                if let Some(ref engine) = llm_engine {
                    let engine = engine.clone();
                    let events_clone = events.clone();
                    let shared_clone = shared.clone();
                    let rid = request_id.clone();

                    tokio::task::spawn_blocking(move || {
                        let result = if stream {
                            let (tx, mut rx) = mpsc::channel::<StreamEvent>(256);
                            let events_for_stream = events_clone.clone();
                            let rid_for_stream = rid.clone();

                            let stream_handle = std::thread::spawn(move || {
                                let mut eng =
                                    tokio::runtime::Handle::current().block_on(engine.lock());
                                if let Some(temp) = temperature {
                                    eng.set_temperature(temp);
                                }
                                let _ = eng.generate_streaming(&prompt, max_tokens, tx);
                            });

                            let consumer = tokio::runtime::Handle::current().spawn(async move {
                                while let Some(event) = rx.recv().await {
                                    match event {
                                        StreamEvent::Token {
                                            token_id,
                                            text,
                                            pos,
                                        } => {
                                            let _ = events_for_stream.send(
                                                IkcEvent::LlmTokenGenerated {
                                                    kernel: KernelId::Kernel03CodeLogicExpert,
                                                    request_id: rid_for_stream.clone(),
                                                    token_id,
                                                    token_text: text,
                                                    position: pos,
                                                },
                                            );
                                        }
                                        StreamEvent::Done {
                                            total_tokens,
                                            tokens_per_second,
                                            full_text,
                                        } => {
                                            let mut state = shared_clone.write().await;
                                            state.code_logic.last_llm_result =
                                                Some(full_text.clone());
                                            state.code_logic.llm_generations_completed += 1;
                                            state.code_logic.llm_total_tokens_generated +=
                                                total_tokens as u64;
                                            state.code_logic.last_token_rate =
                                                Some(tokens_per_second);

                                            let _ = events_for_stream.send(
                                                IkcEvent::LlmGenerationComplete {
                                                    kernel: KernelId::Kernel03CodeLogicExpert,
                                                    request_id: rid_for_stream.clone(),
                                                    total_tokens,
                                                    tokens_per_second,
                                                    full_text,
                                                },
                                            );
                                        }
                                        StreamEvent::Error(msg) => {
                                            let _ = events_for_stream.send(IkcEvent::LlmError {
                                                kernel: KernelId::Kernel03CodeLogicExpert,
                                                request_id: rid_for_stream.clone(),
                                                error_message: msg,
                                            });
                                        }
                                    }
                                }
                            });

                            let _ = tokio::runtime::Handle::current().block_on(consumer);
                            let _ = stream_handle.join();
                            Ok(())
                        } else {
                            let mut eng = tokio::runtime::Handle::current().block_on(engine.lock());
                            if let Some(temp) = temperature {
                                eng.set_temperature(temp);
                            }
                            match eng.generate(&prompt, max_tokens) {
                                Ok(output) => {
                                    let mut state = tokio::runtime::Handle::current()
                                        .block_on(shared_clone.write());
                                    state.code_logic.last_llm_result = Some(output.clone());
                                    state.code_logic.llm_generations_completed += 1;
                                    let _ = events_clone.send(IkcEvent::LlmGenerationComplete {
                                        kernel: KernelId::Kernel03CodeLogicExpert,
                                        request_id: rid.clone(),
                                        total_tokens: output.len(),
                                        tokens_per_second: 0.0,
                                        full_text: output,
                                    });
                                    Ok(())
                                }
                                Err(e) => {
                                    let _ = events_clone.send(IkcEvent::LlmError {
                                        kernel: KernelId::Kernel03CodeLogicExpert,
                                        request_id: rid.clone(),
                                        error_message: e.to_string(),
                                    });
                                    Err(e)
                                }
                            }
                        };

                        if let Err(e) = result {
                            let _ = events_clone.send(IkcEvent::LlmError {
                                kernel: KernelId::Kernel03CodeLogicExpert,
                                request_id: rid,
                                error_message: e.to_string(),
                            });
                        }
                    });
                } else {
                    let _ = events.send(IkcEvent::LlmError {
                        kernel,
                        request_id,
                        error_message: "LLM engine not loaded. Provide a GGUF model file."
                            .to_string(),
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
                        "Kernel_03 rejected unsupported command for Code & Logic Expert: {unsupported:?}"
                    ),
                });
            }
        }

        tokio::task::yield_now().await;
    }

    let _ = events.send(IkcEvent::Stopped { kernel });
}

fn execute_logic_module(module: &str, payload: &str) -> String {
    match module {
        "math.sum" => {
            let values = payload
                .split(|character: char| {
                    !character.is_ascii_digit() && character != '.' && character != '-'
                })
                .filter_map(|candidate| candidate.parse::<f64>().ok())
                .collect::<Vec<_>>();
            let sum = values.iter().sum::<f64>();
            format!(
                "math.sum executed over {} numeric values; sum={sum}",
                values.len()
            )
        }
        "code.analysis" => format!(
            "code.analysis executed; payload_bytes={}, lexical_tokens={}",
            payload.len(),
            payload.split_whitespace().count()
        ),
        "logic.reasoning" => format!(
            "logic.reasoning executed; normalized_assertion='{}'",
            payload.trim().replace('\n', " ")
        ),
        "engineering.expert" => match run_engineering_expert_module(payload) {
            Ok(summary) => summary,
            Err(error) => format!("engineering.expert mmap load failed: {error}"),
        },
        "bio.medical" => match run_bio_medical_module(payload) {
            Ok(summary) => summary,
            Err(error) => format!("bio.medical mmap load failed: {error}"),
        },
        "bio.avian.fastpath" => match run_avian_genetics_fastpath(payload) {
            Ok(summary) => summary,
            Err(error) => format!("bio.avian.fastpath mmap load failed: {error}"),
        },        "finance.math" => match run_finance_math_module(payload) {
            Ok(summary) => summary,
            Err(error) => format!("finance.math mmap load failed: {error}"),
        },
        "global.humanities" => match run_global_humanities_module(payload) {
            Ok(summary) => summary,
            Err(error) => format!("global.humanities mmap load failed: {error}"),
        },
        other => format!(
            "unknown logic module '{other}' isolated safely; payload_bytes={}",
            payload.len()
        ),
    }
}
