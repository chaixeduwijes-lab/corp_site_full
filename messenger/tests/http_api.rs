use std::path::Path;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use messenger_relay::auth;
use messenger_relay::build_router;
use messenger_relay::config::Config;
use messenger_relay::registry::DeviceRegistry;
use messenger_relay::state::{unix_now, AppState, SharedState};

const INVITE: &str = "test-invite-token";

fn test_state() -> SharedState {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        db_path: ":memory:".into(),
        invite_token: Some(INVITE.to_string()),
        message_ttl: Duration::from_secs(60),
        max_queue_per_device: 4,
        max_devices: 3,
        max_body_bytes: 65536,
        tls: None,
    };
    let registry = DeviceRegistry::open(Path::new(":memory:")).unwrap();
    AppState::new(config, registry)
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

fn register_request(sk: &SigningKey, invite_token: &str) -> Request<Body> {
    let pk_b64 = B64.encode(sk.verifying_key().as_bytes());
    let payload = auth::registration_payload(invite_token, &pk_b64);
    let signature = B64.encode(sk.sign(payload.as_bytes()).to_bytes());
    let body = json!({
        "invite_token": invite_token,
        "identity_pk": pk_b64,
        "signature": signature,
    });
    Request::post("/v1/register")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn signed_request(
    sk: &SigningKey,
    device_id: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> Request<Body> {
    let timestamp = unix_now();
    let canonical = auth::canonical_request(method, path, timestamp, body);
    let signature = B64.encode(sk.sign(canonical.as_bytes()).to_bytes());
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("x-device-id", device_id)
        .header("x-timestamp", timestamp.to_string())
        .header("x-signature", signature)
        .body(Body::from(body.to_vec()))
        .unwrap()
}

async fn register(app: &Router, sk: &SigningKey) -> String {
    let (status, body) = call(app, register_request(sk, INVITE)).await;
    assert_eq!(status, StatusCode::CREATED, "register failed: {body}");
    body["device_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn register_send_and_queue_lifecycle() {
    let state = test_state();
    let app = build_router(state.clone());

    let alice = SigningKey::generate(&mut rand::rngs::OsRng);
    let bob = SigningKey::generate(&mut rand::rngs::OsRng);
    let alice_id = register(&app, &alice).await;
    let bob_id = register(&app, &bob).await;

    // The key directory lists both devices.
    let (status, body) = call(
        &app,
        signed_request(&alice, &alice_id, "GET", "/v1/devices", b""),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["devices"].as_array().unwrap().len(), 2);

    // Alice sends an opaque ciphertext to Bob; the relay queues it.
    let send_body =
        json!({ "to": bob_id, "ciphertext": B64.encode(b"opaque-envelope") }).to_string();
    let (status, body) = call(
        &app,
        signed_request(
            &alice,
            &alice_id,
            "POST",
            "/v1/messages",
            send_body.as_bytes(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "send failed: {body}");
    let message_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(state.queue.pending_count(&bob_id), 1);

    // Delivered ciphertext round-trips byte-for-byte, then ACK deletes it.
    let delivered = state.queue.take_undelivered(&bob_id);
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].ciphertext, b"opaque-envelope");
    assert!(state.queue.ack(&bob_id, &message_id));
    assert_eq!(state.queue.pending_count(&bob_id), 0);
}

#[tokio::test]
async fn register_rejects_bad_invite_and_duplicate_key() {
    let state = test_state();
    let app = build_router(state);

    let sk = SigningKey::generate(&mut rand::rngs::OsRng);
    let (status, body) = call(&app, register_request(&sk, "wrong-token")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "invalid_invite_token");

    register(&app, &sk).await;
    let (status, body) = call(&app, register_request(&sk, INVITE)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "duplicate_identity_key");
}

#[tokio::test]
async fn device_limit_is_enforced() {
    let state = test_state(); // max_devices = 3
    let app = build_router(state);

    for _ in 0..3 {
        register(&app, &SigningKey::generate(&mut rand::rngs::OsRng)).await;
    }
    let extra = SigningKey::generate(&mut rand::rngs::OsRng);
    let (status, body) = call(&app, register_request(&extra, INVITE)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "device_limit_reached");
}

#[tokio::test]
async fn authenticated_routes_reject_bad_and_replayed_signatures() {
    let state = test_state();
    let app = build_router(state);

    let alice = SigningKey::generate(&mut rand::rngs::OsRng);
    let alice_id = register(&app, &alice).await;

    // Unsigned request.
    let (status, _) = call(
        &app,
        Request::get("/v1/devices").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Signed by a key that isn't the registered one.
    let mallory = SigningKey::generate(&mut rand::rngs::OsRng);
    let (status, _) = call(
        &app,
        signed_request(&mallory, &alice_id, "GET", "/v1/devices", b""),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A valid signed request works once, then the replay cache rejects it.
    let request = signed_request(&alice, &alice_id, "GET", "/v1/devices", b"");
    let replay = signed_request_clone(&request);
    let (status, _) = call(&app, request).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = call(&app, replay).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "replay accepted: {body}");
}

#[tokio::test]
async fn send_rejects_unknown_recipient_and_enforces_queue_cap() {
    let state = test_state(); // max_queue_per_device = 4
    let app = build_router(state);

    let alice = SigningKey::generate(&mut rand::rngs::OsRng);
    let bob = SigningKey::generate(&mut rand::rngs::OsRng);
    let alice_id = register(&app, &alice).await;
    let bob_id = register(&app, &bob).await;

    let unknown = json!({ "to": "no-such-device", "ciphertext": B64.encode(b"x") }).to_string();
    let (status, body) = call(
        &app,
        signed_request(
            &alice,
            &alice_id,
            "POST",
            "/v1/messages",
            unknown.as_bytes(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "unknown_recipient");

    for i in 0..4 {
        let body = json!({ "to": bob_id, "ciphertext": B64.encode(format!("m{i}").as_bytes()) })
            .to_string();
        let (status, _) = call(
            &app,
            signed_request(&alice, &alice_id, "POST", "/v1/messages", body.as_bytes()),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
    }
    let overflow = json!({ "to": bob_id, "ciphertext": B64.encode(b"m5") }).to_string();
    let (status, body) = call(
        &app,
        signed_request(
            &alice,
            &alice_id,
            "POST",
            "/v1/messages",
            overflow.as_bytes(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"], "queue_full");
}

/// Rebuild an identical request (same headers/body) for replay testing.
fn signed_request_clone(req: &Request<Body>) -> Request<Body> {
    let mut builder = Request::builder().method(req.method()).uri(req.uri());
    for (name, value) in req.headers() {
        builder = builder.header(name, value);
    }
    builder.body(Body::empty()).unwrap()
}
