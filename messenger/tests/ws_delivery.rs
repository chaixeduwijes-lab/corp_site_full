use std::path::Path;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signer, SigningKey};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use messenger_relay::auth;
use messenger_relay::build_router;
use messenger_relay::config::Config;
use messenger_relay::registry::DeviceRegistry;
use messenger_relay::state::{unix_now, AppState, SharedState};

type WsClient = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn spawn_server() -> (SharedState, String) {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        db_path: ":memory:".into(),
        invite_token: Some("invite".to_string()),
        message_ttl: Duration::from_secs(60),
        max_queue_per_device: 16,
        max_devices: 8,
        max_body_bytes: 65536,
        tls: None,
    };
    let registry = DeviceRegistry::open(Path::new(":memory:")).unwrap();
    let state = AppState::new(config, registry);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = build_router(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (state, format!("ws://{addr}/v1/ws"))
}

async fn recv_frame(ws: &mut WsClient) -> Value {
    let deadline = Duration::from_secs(5);
    loop {
        let msg = tokio::time::timeout(deadline, ws.next())
            .await
            .expect("timed out waiting for frame")
            .expect("socket closed")
            .expect("socket error");
        if let Message::Text(text) = msg {
            return serde_json::from_str(text.as_str()).unwrap();
        }
    }
}

async fn connect_authed(url: &str, sk: &SigningKey, device_id: &str) -> WsClient {
    let (mut ws, _) = connect_async(url).await.unwrap();

    let challenge = recv_frame(&mut ws).await;
    assert_eq!(challenge["type"], "challenge");
    let nonce = B64.decode(challenge["nonce"].as_str().unwrap()).unwrap();

    let signature = B64.encode(sk.sign(&auth::ws_auth_payload(&nonce)).to_bytes());
    let auth_frame = json!({ "type": "auth", "device_id": device_id, "signature": signature });
    ws.send(Message::Text(auth_frame.to_string().into()))
        .await
        .unwrap();
    ws
}

#[tokio::test]
async fn ws_authenticates_delivers_and_acks() {
    let (state, url) = spawn_server().await;

    let bob = SigningKey::generate(&mut rand::rngs::OsRng);
    let bob_id = state
        .registry
        .register(bob.verifying_key(), unix_now(), 8)
        .unwrap()
        .device_id;

    // A message queued while Bob is offline is drained right after `ready`.
    let (first_id, _) = state
        .queue
        .enqueue(&bob_id, b"offline-envelope".to_vec(), unix_now())
        .unwrap();

    let mut ws = connect_authed(&url, &bob, &bob_id).await;
    let ready = recv_frame(&mut ws).await;
    assert_eq!(ready["type"], "ready");
    assert_eq!(ready["pending"], 1);

    let delivered = recv_frame(&mut ws).await;
    assert_eq!(delivered["type"], "message");
    assert_eq!(delivered["id"], first_id.as_str());
    assert_eq!(
        B64.decode(delivered["ciphertext"].as_str().unwrap())
            .unwrap(),
        b"offline-envelope"
    );

    ws.send(Message::Text(
        json!({ "type": "ack", "id": first_id }).to_string().into(),
    ))
    .await
    .unwrap();

    // A message queued while Bob is online is pushed live.
    let (second_id, _) = state
        .queue
        .enqueue(&bob_id, b"live-envelope".to_vec(), unix_now())
        .unwrap();
    assert!(state.online.notify(&bob_id));

    let live = recv_frame(&mut ws).await;
    assert_eq!(live["type"], "message");
    assert_eq!(live["id"], second_id.as_str());

    ws.send(Message::Text(
        json!({ "type": "ack", "id": second_id }).to_string().into(),
    ))
    .await
    .unwrap();

    // Both ACKs must eventually empty the queue on the server.
    for _ in 0..50 {
        if state.queue.pending_count(&bob_id) == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(state.queue.pending_count(&bob_id), 0);
}

#[tokio::test]
async fn ws_rejects_bad_signature() {
    let (state, url) = spawn_server().await;

    let bob = SigningKey::generate(&mut rand::rngs::OsRng);
    let bob_id = state
        .registry
        .register(bob.verifying_key(), unix_now(), 8)
        .unwrap()
        .device_id;

    // Mallory knows Bob's device id but not his key.
    let mallory = SigningKey::generate(&mut rand::rngs::OsRng);
    let mut ws = connect_authed(&url, &mallory, &bob_id).await;

    let reply = recv_frame(&mut ws).await;
    assert_eq!(reply["type"], "error");
    assert_eq!(reply["reason"], "unauthorized");
}

#[tokio::test]
async fn second_connection_replaces_first() {
    let (state, url) = spawn_server().await;

    let bob = SigningKey::generate(&mut rand::rngs::OsRng);
    let bob_id = state
        .registry
        .register(bob.verifying_key(), unix_now(), 8)
        .unwrap()
        .device_id;

    let mut first = connect_authed(&url, &bob, &bob_id).await;
    assert_eq!(recv_frame(&mut first).await["type"], "ready");

    let mut second = connect_authed(&url, &bob, &bob_id).await;
    assert_eq!(recv_frame(&mut second).await["type"], "ready");

    let evicted = recv_frame(&mut first).await;
    assert_eq!(evicted["type"], "error");
    assert_eq!(evicted["reason"], "replaced");
}
