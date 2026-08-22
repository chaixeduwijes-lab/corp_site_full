use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::auth;
use crate::state::{SharedState, WakeEvent};

const AUTH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientFrame {
    Auth {
        device_id: String,
        signature: String,
    },
    Ack {
        id: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerFrame {
    Challenge {
        nonce: String,
    },
    Ready {
        pending: usize,
    },
    Message {
        id: String,
        ciphertext: String,
        queued_at: u64,
        expires_at: u64,
    },
    Error {
        reason: &'static str,
    },
}

pub async fn ws_handler(State(state): State<SharedState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: SharedState, socket: WebSocket) {
    let (mut tx, mut rx) = socket.split();

    // Challenge-response authentication: the client proves possession of its
    // registered Ed25519 key by signing a fresh random nonce.
    let mut nonce = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    if send_frame(
        &mut tx,
        &ServerFrame::Challenge {
            nonce: B64.encode(nonce),
        },
    )
    .await
    .is_err()
    {
        return;
    }

    let auth_frame = match timeout(AUTH_TIMEOUT, next_client_frame(&mut rx)).await {
        Ok(Some(frame)) => frame,
        _ => {
            let _ = send_frame(
                &mut tx,
                &ServerFrame::Error {
                    reason: "auth_timeout",
                },
            )
            .await;
            return;
        }
    };

    let device = match auth_frame {
        ClientFrame::Auth {
            device_id,
            signature,
        } => {
            let device = state.registry.get(&device_id);
            let ok = device.as_ref().is_some_and(|d| {
                auth::verify_signature(&d.verifying_key, &auth::ws_auth_payload(&nonce), &signature)
                    .is_ok()
            });
            if !ok {
                let _ = send_frame(
                    &mut tx,
                    &ServerFrame::Error {
                        reason: "unauthorized",
                    },
                )
                .await;
                return;
            }
            device.unwrap()
        }
        _ => {
            let _ = send_frame(
                &mut tx,
                &ServerFrame::Error {
                    reason: "auth_required",
                },
            )
            .await;
            return;
        }
    };

    let device_id = device.device_id;
    let conn_id = uuid::Uuid::new_v4().to_string();
    let (wake_tx, mut wake_rx) = mpsc::unbounded_channel::<WakeEvent>();
    state.online.connect(&device_id, &conn_id, wake_tx);
    tracing::info!(device_id = %device_id, "websocket connected");

    let pending = state.queue.pending_count(&device_id);
    let mut open = send_frame(&mut tx, &ServerFrame::Ready { pending })
        .await
        .is_ok();

    if open {
        open = deliver_pending(&state, &device_id, &mut tx).await;
    }

    while open {
        tokio::select! {
            wake = wake_rx.recv() => match wake {
                Some(WakeEvent::NewMessage) => {
                    open = deliver_pending(&state, &device_id, &mut tx).await;
                }
                Some(WakeEvent::Replaced) => {
                    let _ = send_frame(&mut tx, &ServerFrame::Error { reason: "replaced" }).await;
                    break;
                }
                None => break,
            },
            incoming = rx.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(ClientFrame::Ack { id }) = serde_json::from_str(text.as_str()) {
                        state.queue.ack(&device_id, &id);
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
            },
        }
    }

    state.online.disconnect(&device_id, &conn_id);
    state.queue.reset_delivered(&device_id);
    tracing::info!(device_id = %device_id, "websocket disconnected");
}

/// Push every not-yet-pushed queued message. Returns false once the socket
/// is no longer writable.
async fn deliver_pending(
    state: &SharedState,
    device_id: &str,
    tx: &mut SplitSink<WebSocket, Message>,
) -> bool {
    for msg in state.queue.take_undelivered(device_id) {
        let frame = ServerFrame::Message {
            id: msg.id,
            ciphertext: B64.encode(&msg.ciphertext),
            queued_at: msg.queued_at,
            expires_at: msg.expires_at,
        };
        if send_frame(tx, &frame).await.is_err() {
            return false;
        }
    }
    true
}

async fn send_frame(
    tx: &mut SplitSink<WebSocket, Message>,
    frame: &ServerFrame,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(frame).expect("server frame serializes");
    tx.send(Message::Text(text.into())).await
}

async fn next_client_frame(
    rx: &mut futures_util::stream::SplitStream<WebSocket>,
) -> Option<ClientFrame> {
    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(text)) => return serde_json::from_str(text.as_str()).ok(),
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(_) => continue,
        }
    }
    None
}
