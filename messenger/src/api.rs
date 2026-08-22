use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{self, AuthError};
use crate::registry::RegisterError;
use crate::state::{unix_now, SharedState};

pub const REGISTER_PATH: &str = "/v1/register";
pub const DEVICES_PATH: &str = "/v1/devices";
pub const MESSAGES_PATH: &str = "/v1/messages";

/// API error: a status code and a stable machine-readable reason. Reasons are
/// deliberately terse — the server explains nothing it doesn't have to.
pub struct ApiError(StatusCode, &'static str);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<AuthError> for ApiError {
    fn from(_: AuthError) -> Self {
        // One opaque answer for every auth failure mode.
        ApiError(StatusCode::UNAUTHORIZED, "unauthorized")
    }
}

pub async fn healthz() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
struct RegisterRequest {
    invite_token: String,
    /// Base64-encoded Ed25519 public key (32 bytes).
    identity_pk: String,
    /// Base64 signature over `registration_payload(invite_token, identity_pk)`,
    /// proving possession of the private key.
    signature: String,
}

#[derive(Serialize)]
struct RegisterResponse {
    device_id: String,
    created_at: u64,
}

pub async fn register(
    State(state): State<SharedState>,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let Some(expected_token) = state.config.invite_token.as_deref() else {
        return Err(ApiError(StatusCode::FORBIDDEN, "registration_disabled"));
    };

    let req: RegisterRequest = serde_json::from_slice(&body)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_json"))?;

    if !auth::ct_eq(req.invite_token.as_bytes(), expected_token.as_bytes()) {
        return Err(ApiError(StatusCode::FORBIDDEN, "invalid_invite_token"));
    }

    let pk_bytes: [u8; 32] = B64
        .decode(&req.identity_pk)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "invalid_identity_pk"))?;
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_identity_pk"))?;

    let payload = auth::registration_payload(&req.invite_token, &req.identity_pk);
    auth::verify_signature(&verifying_key, payload.as_bytes(), &req.signature)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_signature"))?;

    let device = state
        .registry
        .register(verifying_key, unix_now(), state.config.max_devices)
        .map_err(|e| match e {
            RegisterError::DuplicateKey => ApiError(StatusCode::CONFLICT, "duplicate_identity_key"),
            RegisterError::LimitReached => ApiError(StatusCode::CONFLICT, "device_limit_reached"),
        })?;

    tracing::info!(device_id = %device.device_id, "device registered");
    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            device_id: device.device_id,
            created_at: device.created_at,
        }),
    ))
}

#[derive(Serialize)]
struct DeviceEntry {
    device_id: String,
    identity_pk: String,
    created_at: u64,
}

pub async fn list_devices(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    auth::verify_request(&state, &headers, "GET", DEVICES_PATH, &body, unix_now())?;

    let devices: Vec<DeviceEntry> = state
        .registry
        .list()
        .into_iter()
        .map(|d| DeviceEntry {
            device_id: d.device_id,
            identity_pk: B64.encode(d.verifying_key.as_bytes()),
            created_at: d.created_at,
        })
        .collect();

    Ok(Json(json!({ "devices": devices })))
}

#[derive(Deserialize)]
struct SendRequest {
    /// Recipient device id.
    to: String,
    /// Base64-encoded opaque E2EE envelope. The relay never interprets it.
    ciphertext: String,
}

pub async fn send_message(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    auth::verify_request(&state, &headers, "POST", MESSAGES_PATH, &body, unix_now())?;

    let req: SendRequest = serde_json::from_slice(&body)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_json"))?;

    let ciphertext = B64
        .decode(&req.ciphertext)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_ciphertext"))?;
    if ciphertext.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "invalid_ciphertext"));
    }

    if state.registry.get(&req.to).is_none() {
        return Err(ApiError(StatusCode::NOT_FOUND, "unknown_recipient"));
    }

    let (id, expires_at) = state
        .queue
        .enqueue(&req.to, ciphertext, unix_now())
        .map_err(|_| ApiError(StatusCode::TOO_MANY_REQUESTS, "queue_full"))?;

    state.online.notify(&req.to);
    tracing::debug!(message_id = %id, "ciphertext queued");

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "id": id, "expires_at": expires_at })),
    ))
}
