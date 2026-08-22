//! End-to-end proof: two reference clients exchange messages through a real
//! `messenger-relay` instance, and the relay only ever holds opaque ciphertext.
//!
//! The relay runs in-process on a real TCP socket, so the clients talk to it
//! over genuine HTTP + WebSocket — the same path a phone would use.

use std::path::Path;
use std::time::Duration;

use messenger_client::{wire, Client};
use messenger_relay::config::Config;
use messenger_relay::registry::DeviceRegistry;
use messenger_relay::state::{AppState, SharedState};

const INVITE: &str = "e2e-invite";

async fn spawn_relay() -> (SharedState, String) {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        db_path: ":memory:".into(),
        invite_token: Some(INVITE.to_string()),
        message_ttl: Duration::from_secs(60),
        max_queue_per_device: 64,
        max_devices: 16,
        max_body_bytes: 65536,
        max_total_queue_bytes: 1 << 20,
        memory_hardening: false,
        tls: None,
    };
    let registry = DeviceRegistry::open(Path::new(":memory:")).unwrap();
    let state = AppState::new(config, registry);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = messenger_relay::build_router(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (state, format!("http://{addr}"))
}

async fn recv_timeout(client: &mut Client) -> messenger_client::Received {
    tokio::time::timeout(Duration::from_secs(5), client.recv())
        .await
        .expect("recv timed out")
        .expect("recv error")
        .expect("connection closed")
}

#[tokio::test]
async fn plaintext_never_reaches_the_relay() {
    let (state, url) = spawn_relay().await;

    let mut alice = Client::register(&url, INVITE).await.unwrap();
    let mut bob = Client::register(&url, INVITE).await.unwrap();
    bob.publish_prekeys(10).await.unwrap();

    let secret = "meet at the docks at midnight";
    alice.send_text(&bob.device_id, secret).await.unwrap();

    // Inspect exactly what the relay is holding for Bob: it must be one opaque
    // envelope that (a) does NOT contain the plaintext and (b) is a well-formed
    // Olm message the relay itself cannot interpret.
    let queued = state.queue.peek(&bob.device_id);
    assert_eq!(queued.len(), 1, "relay should hold exactly one envelope");
    assert!(
        !wire::contains(&queued[0], secret.as_bytes()),
        "plaintext leaked into the relay-visible ciphertext!"
    );
    assert!(
        wire::unpack(&queued[0]).is_ok(),
        "relay bytes should be a valid opaque Olm message"
    );

    // Bob connects and decrypts end-to-end.
    bob.connect().await.unwrap();
    let got = recv_timeout(&mut bob).await;
    assert_eq!(got.plaintext, secret);
    assert_eq!(
        got.sender_identity,
        alice.identity_key(),
        "message attributed to Alice's end-to-end identity"
    );

    // Relay deleted the message after Bob's ACK.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(state.queue.pending_count(&bob.device_id), 0);
}

#[tokio::test]
async fn bidirectional_conversation() {
    let (_state, url) = spawn_relay().await;

    let mut alice = Client::register(&url, INVITE).await.unwrap();
    let mut bob = Client::register(&url, INVITE).await.unwrap();
    alice.publish_prekeys(10).await.unwrap();
    bob.publish_prekeys(10).await.unwrap();

    alice.connect().await.unwrap();
    bob.connect().await.unwrap();

    // Alice -> Bob (establishes the session via a pre-key message).
    alice.send_text(&bob.device_id, "ping").await.unwrap();
    let at_bob = recv_timeout(&mut bob).await;
    assert_eq!(at_bob.plaintext, "ping");
    assert_eq!(at_bob.sender_identity, alice.identity_key());

    // Bob -> Alice (normal ratchet message on the established session).
    bob.send_text(&alice.device_id, "pong").await.unwrap();
    let at_alice = recv_timeout(&mut alice).await;
    assert_eq!(at_alice.plaintext, "pong");
    assert_eq!(at_alice.sender_identity, bob.identity_key());

    // A second round to exercise the ratchet advancing on both sides.
    alice.send_text(&bob.device_id, "ping-2").await.unwrap();
    assert_eq!(recv_timeout(&mut bob).await.plaintext, "ping-2");
    bob.send_text(&alice.device_id, "pong-2").await.unwrap();
    assert_eq!(recv_timeout(&mut alice).await.plaintext, "pong-2");
}
