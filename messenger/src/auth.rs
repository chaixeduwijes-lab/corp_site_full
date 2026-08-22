use std::collections::HashMap;
use std::sync::Mutex;

use axum::http::HeaderMap;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::config::AUTH_WINDOW_SECS;
use crate::registry::Device;
use crate::state::AppState;

/// Canonical string a client signs for an authenticated HTTP request.
/// `path` is the literal route path (no query string).
pub fn canonical_request(method: &str, path: &str, timestamp: u64, body: &[u8]) -> String {
    let body_hash = hex::encode(Sha256::digest(body));
    format!("v1|{method}|{path}|{timestamp}|{body_hash}")
}

/// Payload signed during registration; binds the invite token to the key so a
/// captured registration request cannot be replayed with a different key.
pub fn registration_payload(invite_token: &str, identity_pk_b64: &str) -> String {
    format!("register|v1|{invite_token}|{identity_pk_b64}")
}

/// Payload signed to authenticate a WebSocket connection.
pub fn ws_auth_payload(nonce: &[u8]) -> Vec<u8> {
    [b"ws-auth|v1|".as_slice(), nonce].concat()
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    MissingHeaders,
    StaleTimestamp,
    UnknownDevice,
    BadSignature,
    Replayed,
}

/// Verify the Ed25519 request signature carried in `x-device-id`,
/// `x-timestamp` and `x-signature` headers.
pub fn verify_request(
    state: &AppState,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    body: &[u8],
    now: u64,
) -> Result<Device, AuthError> {
    let device_id = header_str(headers, "x-device-id").ok_or(AuthError::MissingHeaders)?;
    let timestamp: u64 = header_str(headers, "x-timestamp")
        .and_then(|t| t.parse().ok())
        .ok_or(AuthError::MissingHeaders)?;
    let signature_b64 = header_str(headers, "x-signature").ok_or(AuthError::MissingHeaders)?;

    if now.abs_diff(timestamp) > AUTH_WINDOW_SECS {
        return Err(AuthError::StaleTimestamp);
    }

    let device = state
        .registry
        .get(device_id)
        .ok_or(AuthError::UnknownDevice)?;
    let canonical = canonical_request(method, path, timestamp, body);
    verify_signature(&device.verifying_key, canonical.as_bytes(), signature_b64)?;

    // A valid signature may only be presented once within the auth window.
    if !state.replay.check_and_insert(signature_b64, now) {
        return Err(AuthError::Replayed);
    }

    Ok(device)
}

pub fn verify_signature(
    key: &VerifyingKey,
    message: &[u8],
    signature_b64: &str,
) -> Result<(), AuthError> {
    let sig_bytes = B64
        .decode(signature_b64)
        .map_err(|_| AuthError::BadSignature)?;
    let signature = Signature::from_slice(&sig_bytes).map_err(|_| AuthError::BadSignature)?;
    key.verify_strict(message, &signature)
        .map_err(|_| AuthError::BadSignature)
}

/// Constant-time byte comparison for the invite token.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

/// Remembers recently seen request signatures so a captured signed request
/// cannot be replayed within the timestamp window.
#[derive(Default)]
pub struct ReplayCache {
    seen: Mutex<HashMap<String, u64>>,
}

impl ReplayCache {
    /// Returns false if the signature was already seen and not yet expired.
    pub fn check_and_insert(&self, signature: &str, now: u64) -> bool {
        let mut seen = self.seen.lock().unwrap();
        match seen.get(signature) {
            Some(&expires_at) if expires_at > now => false,
            _ => {
                seen.insert(signature.to_string(), now + AUTH_WINDOW_SECS + 1);
                true
            }
        }
    }

    pub fn sweep(&self, now: u64) {
        self.seen.lock().unwrap().retain(|_, &mut exp| exp > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn signature_roundtrip_and_tamper_detection() {
        let sk = SigningKey::generate(&mut rand::rngs::OsRng);
        let canonical = canonical_request("POST", "/v1/messages", 1000, b"{}");
        let sig = B64.encode(sk.sign(canonical.as_bytes()).to_bytes());

        assert!(verify_signature(&sk.verifying_key(), canonical.as_bytes(), &sig).is_ok());

        let tampered = canonical_request("POST", "/v1/messages", 1001, b"{}");
        assert_eq!(
            verify_signature(&sk.verifying_key(), tampered.as_bytes(), &sig),
            Err(AuthError::BadSignature)
        );
    }

    #[test]
    fn replay_cache_rejects_second_use_until_expiry() {
        let cache = ReplayCache::default();
        assert!(cache.check_and_insert("sig", 1000));
        assert!(!cache.check_and_insert("sig", 1000));
        // After the window passes the entry is stale and may be evicted.
        assert!(cache.check_and_insert("sig", 1000 + AUTH_WINDOW_SECS + 2));
    }

    #[test]
    fn ct_eq_basic() {
        assert!(ct_eq(b"token", b"token"));
        assert!(!ct_eq(b"token", b"Token"));
        assert!(!ct_eq(b"token", b"tokens"));
    }
}
