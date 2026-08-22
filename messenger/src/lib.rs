//! messenger-relay: a ciphertext-only message relay for a closed-group E2EE
//! messenger.
//!
//! Security model (see ../docs/anonymous-vps-e2ee-messenger.md and the audit
//! in ../docs/security-audit-messenger-relay.md):
//! - the server only ever sees opaque ciphertext envelopes — it is
//!   zero-*content*-knowledge, not zero-knowledge: it still observes routing
//!   metadata (sender, recipient, timing), whose minimisation is future work;
//! - queued ciphertext lives only in RAM and is never written to disk; that
//!   guarantee holds only with the memory hardening in `harden.rs` applied
//!   (locked pages, no core dumps) plus swap disabled/encrypted on the host;
//! - messages are deleted on ACK or after a TTL hard-capped at 20 minutes, and
//!   their buffers are zeroized on removal (best-effort; the real
//!   crypto-erasure is the clients' ratchet discarding message keys);
//! - the only persistent data is the device directory: opaque device ids and
//!   Ed25519 *public* identity keys;
//! - all E2EE private keys live on the users' devices, never here.

pub mod api;
pub mod auth;
pub mod config;
pub mod harden;
pub mod prekeys;
pub mod queue;
pub mod registry;
pub mod state;
pub mod ws;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;

use crate::state::SharedState;

pub fn build_router(state: SharedState) -> Router {
    let max_body = state.config.max_body_bytes;
    Router::new()
        .route("/healthz", get(api::healthz))
        .route(api::REGISTER_PATH, post(api::register))
        .route(api::DEVICES_PATH, get(api::list_devices))
        .route(api::MESSAGES_PATH, post(api::send_message))
        .route(api::PREKEYS_PATH, post(api::publish_prekeys))
        .route("/v1/prekeys/{device_id}", get(api::claim_prekeys))
        .route("/v1/ws", get(ws::ws_handler))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state)
}
