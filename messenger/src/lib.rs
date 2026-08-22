//! messenger-relay: a zero-knowledge message relay for a closed-group E2EE
//! messenger.
//!
//! Security model (see ../docs/anonymous-vps-e2ee-messenger.md):
//! - the server only ever sees opaque ciphertext envelopes;
//! - the message queue lives exclusively in RAM — nothing message-related
//!   touches the disk, so a disk snapshot yields no message data;
//! - messages are deleted on ACK or after a TTL hard-capped at 20 minutes;
//! - the only persistent data is the device directory: opaque device ids and
//!   Ed25519 *public* identity keys;
//! - all E2EE private keys live on the users' devices, never here.

pub mod api;
pub mod auth;
pub mod config;
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
        .route("/v1/ws", get(ws::ws_handler))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state)
}
