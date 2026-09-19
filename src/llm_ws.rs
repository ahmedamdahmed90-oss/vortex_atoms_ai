use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State as AxumState, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::llm_api::ApiState;
use crate::llm_stream::StreamEvent;
use crate::security::AuthRole;

#[derive(Deserialize)]
pub struct WsGenerateRequest {
    pub prompt: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    #[serde(rename = "token")]
    Token {
        token_id: u32,
        text: String,
        position: usize,
    },
    #[serde(rename = "done")]
    Done {
        total_tokens: usize,
        tokens_per_second: f64,
        full_text: String,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "ready")]
    Ready {},
    #[serde(rename = "batch")]
    Batch { events: Vec<WsEvent> },
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    AxumState(state): AxumState<Arc<RwLock<ApiState>>>,
    role: Option<Extension<AuthRole>>,
) -> impl IntoResponse {
    // The auth middleware attaches a role to every authorized request; a
    // missing extension means the middleware was bypassed — fail closed.
    if role.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "missing or invalid API token"})),
        )
            .into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
        .into_response()
}

/// Server-side socket bounds: idle sockets are reaped, oversized frames are
/// rejected, and the server pings so dead peers are detected promptly.
const WS_IDLE_TIMEOUT_SECS: u64 = 300;
const WS_PING_INTERVAL_SECS: u64 = 60;
const WS_MAX_TEXT_BYTES: usize = 262_144;

async fn handle_socket(socket: WebSocket, state: Arc<RwLock<ApiState>>) {
    let (mut sender, mut receiver) = socket.split();

    let ready = serde_json::to_string(&WsEvent::Ready {}).unwrap_or_default();
    if sender.send(Message::Text(ready.into())).await.is_err() {
        return;
    }

    let mut pinger = tokio::time::interval(std::time::Duration::from_secs(WS_PING_INTERVAL_SECS));
    pinger.tick().await; // first tick fires immediately; skip it.

    loop {
        tokio::select! {
            _ = pinger.tick() => {
                // Any send failure means the peer is gone.
                if sender.send(Message::Ping(Vec::<u8>::new().into())).await.is_err() {
                    break;
                }
            }
            incoming = tokio::time::timeout(
                std::time::Duration::from_secs(WS_IDLE_TIMEOUT_SECS),
                receiver.next(),
            ) => {
                let msg = match incoming {
                    // Idle too long: reap the socket.
                    Err(_) => break,
                    Ok(None) => break,
                    Ok(Some(Err(_))) => break,
                    Ok(Some(Ok(m))) => m,
                };
                // Any frame (including Ping/Pong) counts as activity.
                match msg {
                    Message::Close(_) => break,
                    Message::Ping(_) | Message::Pong(_) => continue,
                    _ => {}
                }
                if let Message::Text(ref text) = msg {
                    if text.len() > WS_MAX_TEXT_BYTES {
                        let _ = sender.close().await;
                        break;
                    }
                }
                if !handle_ws_message(msg, &mut sender, &state).await {
                    break;
                }
            }
        }
    }
}

/// Serialize and send a single terminal `WsEvent` immediately. Any buffered
/// token batches are flushed by the caller first (order preserved), so a
/// terminal frame always lands after the tokens that precede it.
/// Serialize and send one coalesced batch of buffered token events (order
/// preserved). A fresh borrow of atch is taken per call and released at
/// .await, so call sites may also inspect atch.is_empty() in between.
async fn send_batch(
    batch: &mut Vec<WsEvent>,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> bool {
    let json = serde_json::to_string(&WsEvent::Batch {
        events: std::mem::take(batch),
    })
    .unwrap_or_default();
    sender.send(Message::Text(json.into())).await.is_ok()
}
async fn send_terminal(
    ev: &StreamEvent,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> bool {
    let frame = match ev {
        StreamEvent::Done {
            total_tokens,
            tokens_per_second,
            full_text,
        } => WsEvent::Done {
            total_tokens: *total_tokens,
            tokens_per_second: *tokens_per_second,
            full_text: full_text.clone(),
        },
        StreamEvent::Error(msg) => WsEvent::Error {
            message: msg.clone(),
        },
        // Token events never reach the terminal path.
        StreamEvent::Token { .. } => return false,
    };
    let json = serde_json::to_string(&frame).unwrap_or_default();
    sender.send(Message::Text(json.into())).await.is_ok()
}
async fn handle_ws_message(
    msg: Message,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    state: &Arc<RwLock<ApiState>>,
) -> bool {
    match msg {
        Message::Text(text) => {
            let request: WsGenerateRequest = match serde_json::from_str(&text) {
                Ok(r) => r,
                Err(e) => {
                    let err = serde_json::to_string(&WsEvent::Error {
                        message: format!("Invalid request: {e}"),
                    })
                    .unwrap_or_default();
                    let _ = sender.send(Message::Text(err.into())).await;
                    return true;
                }
            };

            let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamEvent>(256);

            // Same admission control as the REST inference routes: without
            // this, WS clients could pile unbounded blocking compute onto
            // the serial engine and starve every other endpoint.
            let permit = match crate::llm_api::acquire_inference_permit(state).await {
                Ok(p) => p,
                Err(_) => {
                    let err = serde_json::to_string(&WsEvent::Error {
                        message: "engine busy: too many concurrent inference requests, retry later"
                            .to_string(),
                    })
                    .unwrap_or_default();
                    let _ = sender.send(Message::Text(err.into())).await;
                    return true;
                }
            };

            let (engine_arc, temp_cfg) = {
                let state_guard = state.read().await;
                let snap = state_guard.security.cfg_snapshot();
                (
                    state_guard.engine.clone(),
                    (
                        snap.temperature_min,
                        snap.temperature_max,
                        snap.max_prompt_chars,
                    ),
                )
            };

            if let Err(e) = crate::security::check_text_len(&request.prompt, temp_cfg.2, "prompt") {
                let err = serde_json::to_string(&WsEvent::Error { message: e }).unwrap_or_default();
                let _ = sender.send(Message::Text(err.into())).await;
                return true;
            }

            let temperature = match request.temperature {
                Some(t) if !t.is_finite() => {
                    let err = serde_json::to_string(&WsEvent::Error {
                        message: "temperature must be a finite number".to_string(),
                    })
                    .unwrap_or_default();
                    let _ = sender.send(Message::Text(err.into())).await;
                    return true;
                }
                Some(t) => Some(t.clamp(temp_cfg.0, temp_cfg.1)),
                None => None,
            };

            let prompt = request.prompt.clone();
            let max_tokens = crate::llm_api::clamp_max_tokens(request.max_tokens);

            tokio::task::spawn_blocking(move || {
                // Held for the whole generation: bounds WS concurrency.
                let _permit = permit;
                let mut engine = engine_arc.blocking_write();

                if let Some(temp) = temperature {
                    engine.set_temperature(temp);
                }

                if let Err(e) = engine.generate_streaming(&prompt, Some(max_tokens), tx) {
                    eprintln!("[WS] Generation error: {}", e.public_message());
                }
            });

            // PERF-02 §6.1 — token coalescing window: token events are buffered
            // and flushed at most once per `ws_coalesce_ms` (order preserved),
            // terminal events flush immediately. One disk read per socket —
            // reflects the current vortex.json (0 = classic per-token frames).
            let coalesce_ms = crate::vortex_config::VortexConfig::load()
                .performance
                .ws_coalesce_ms;
            let coalesce = std::time::Duration::from_millis(coalesce_ms);

            let mut batch: Vec<WsEvent> = Vec::new();
            let mut flushed = std::time::Instant::now();

            loop {
                let window_remaining = coalesce
                    .checked_sub(flushed.elapsed())
                    .unwrap_or(std::time::Duration::ZERO);
                let next = if coalesce_ms == 0 {
                    rx.recv().await
                } else {
                    match tokio::time::timeout(window_remaining, rx.recv()).await {
                        Ok(opt) => opt,
                        // Window closed with nothing new: flush what we have.
                        Err(_) => {
                            if !batch.is_empty() && !send_batch(&mut batch, sender).await {
                                return false;
                            }
                            flushed = std::time::Instant::now();
                            continue;
                        }
                    }
                };

                let ev = match next {
                    // Engine ended the stream.
                    Some(ev) => ev,
                    _ => {
                        if !batch.is_empty() {
                            let _ = send_batch(&mut batch, sender).await;
                        }
                        break;
                    }
                };

                match ev {
                    StreamEvent::Token {
                        token_id,
                        text,
                        pos,
                    } => {
                        batch.push(WsEvent::Token {
                            token_id,
                            text,
                            position: pos,
                        });
                        // Very fast serial engines could fill the window every
                        // token; cap frame latency to one coalescing window.
                        if !batch.is_empty()
                            && (coalesce_ms == 0 || flushed.elapsed() >= coalesce)
                            && !send_batch(&mut batch, sender).await
                        {
                            return false;
                        }
                    }
                    terminal => {
                        // Terminal event: flush buffered tokens first (order
                        // preserved), then the terminal event immediately.
                        if !batch.is_empty() && !send_batch(&mut batch, sender).await {
                            return false;
                        }
                        if !send_terminal(&terminal, sender).await {
                            return false;
                        }
                        break;
                    }
                }
            }
            true
        }
        // Unreachable: Ping/Pong/Close/Binary frames are filtered by the
        // caller loop above; anything else keeps the socket open.
        _ => true,
    }
}
