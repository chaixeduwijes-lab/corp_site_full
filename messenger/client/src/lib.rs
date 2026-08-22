//! Reference E2EE client for `messenger-relay`.
//!
//! This client demonstrates the intended end-to-end property: plaintext exists
//! only on the endpoints, and the relay carries opaque bytes. All cryptography
//! is delegated to [`vodozemac`] (an audited implementation of the Olm
//! X3DH-style handshake + Double Ratchet) — this crate never rolls its own,
//! per the audit's own guidance.
//!
//! Two independent key pairs are used, deliberately:
//! - an **Ed25519 relay transport key** (this crate, via `ed25519-dalek`) that
//!   authenticates HTTP/WebSocket requests to the relay; and
//! - a **vodozemac account** holding the messaging identity + ratchet state.
//!
//! The relay only ever learns the transport public key and the (public)
//! prekey bundle. Private keys — transport and messaging alike — never leave
//! the client.

use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signer, SigningKey};
use futures_util::{SinkExt, StreamExt};
use messenger_relay::auth;
use rand::rngs::OsRng;
use serde::Deserialize;
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use vodozemac::olm::{Account, OlmMessage, SessionConfig};
use vodozemac::Curve25519PublicKey;

pub mod wire;
pub use wire::{pack, unpack};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A decrypted inbound message.
#[derive(Debug, Clone)]
pub struct Received {
    /// The sender's messaging identity key (base64 Curve25519). The relay
    /// never reveals sender device ids, so this — extracted end-to-end from
    /// the ciphertext — is how the app attributes a message.
    pub sender_identity: String,
    pub plaintext: String,
    /// Relay message id, needed to ACK.
    pub message_id: String,
}

pub struct Client {
    base_url: String,
    http: reqwest::Client,
    signing_key: SigningKey,
    pub device_id: String,
    account: Account,
    /// Established sessions keyed by the peer's Curve25519 identity (base64).
    sessions: HashMap<String, vodozemac::olm::Session>,
    /// Cache of peer device id -> Curve25519 identity, learned when claiming
    /// a peer's prekey bundle.
    peer_identity: HashMap<String, String>,
    ws: Option<Ws>,
}

impl Client {
    /// Register a fresh device with the relay and return a ready client.
    pub async fn register(base_url: &str, invite_token: &str) -> Result<Self> {
        let signing_key = SigningKey::generate(&mut OsRng);
        let account = Account::new();

        let http = reqwest::Client::builder()
            .build()
            .context("build http client")?;
        let pk_b64 = B64.encode(signing_key.verifying_key().as_bytes());
        let payload = auth::registration_payload(invite_token, &pk_b64);
        let signature = B64.encode(signing_key.sign(payload.as_bytes()).to_bytes());

        let resp = http
            .post(format!("{base_url}/v1/register"))
            .json(&json!({
                "invite_token": invite_token,
                "identity_pk": pk_b64,
                "signature": signature,
            }))
            .send()
            .await
            .context("register request")?;
        if !resp.status().is_success() {
            bail!(
                "register failed: {} {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            );
        }
        #[derive(Deserialize)]
        struct RegisterResponse {
            device_id: String,
        }
        let device_id = resp.json::<RegisterResponse>().await?.device_id;

        Ok(Self {
            base_url: base_url.to_string(),
            http,
            signing_key,
            device_id,
            account,
            sessions: HashMap::new(),
            peer_identity: HashMap::new(),
            ws: None,
        })
    }

    /// This client's messaging identity key (base64 Curve25519).
    pub fn identity_key(&self) -> String {
        self.account.curve25519_key().to_base64()
    }

    /// Generate `count` one-time keys plus a fallback key and publish the
    /// resulting bundle so peers can start sessions while this client is
    /// offline.
    pub async fn publish_prekeys(&mut self, count: usize) -> Result<()> {
        self.account.generate_one_time_keys(count);
        self.account.generate_fallback_key();

        let identity_key = self.account.curve25519_key().to_base64();
        let one_time_keys: Vec<_> = self
            .account
            .one_time_keys()
            .into_iter()
            .map(|(id, key)| json!({ "id": id.to_base64(), "key": key.to_base64() }))
            .collect();
        let fallback_key = self
            .account
            .fallback_key()
            .into_values()
            .next()
            .map(|k| k.to_base64());

        let body = serde_json::to_vec(&json!({
            "identity_key": identity_key,
            "one_time_keys": one_time_keys,
            "fallback_key": fallback_key,
        }))?;

        let resp = self.signed("POST", "/v1/prekeys", body).send().await?;
        if !resp.status().is_success() {
            bail!("publish_prekeys failed: {}", resp.status());
        }
        self.account.mark_keys_as_published();
        Ok(())
    }

    /// Encrypt `text` to `peer_device_id` and hand the ciphertext to the relay.
    /// Establishes an outbound session on first contact by claiming one of the
    /// peer's prekeys.
    pub async fn send_text(&mut self, peer_device_id: &str, text: &str) -> Result<()> {
        let peer_identity = self.ensure_session(peer_device_id).await?;
        let session = self
            .sessions
            .get_mut(&peer_identity)
            .expect("session established");
        let olm = session.encrypt(text).context("encrypt")?;
        let wire = wire::pack(&olm);

        let body = serde_json::to_vec(&json!({
            "to": peer_device_id,
            "ciphertext": B64.encode(&wire),
        }))?;
        let resp = self.signed("POST", "/v1/messages", body).send().await?;
        if !resp.status().is_success() {
            bail!("send failed: {}", resp.status());
        }
        Ok(())
    }

    /// Ensure an outbound session to a peer exists, returning the peer's
    /// Curve25519 identity (the session map key).
    async fn ensure_session(&mut self, peer_device_id: &str) -> Result<String> {
        if let Some(identity) = self.peer_identity.get(peer_device_id) {
            if self.sessions.contains_key(identity) {
                return Ok(identity.clone());
            }
        }

        // Claim a prekey bundle for the peer (consumes one of their one-time
        // keys). The path carries the target device id and is what we sign.
        let path = format!("/v1/prekeys/{peer_device_id}");
        let resp = self.signed("GET", &path, Vec::new()).send().await?;
        if !resp.status().is_success() {
            bail!("no prekeys for peer: {}", resp.status());
        }
        #[derive(Deserialize)]
        struct OneTimeKey {
            #[allow(dead_code)]
            id: String,
            key: String,
        }
        #[derive(Deserialize)]
        struct ClaimedBundle {
            identity_key: String,
            one_time_key: Option<OneTimeKey>,
            fallback_key: Option<String>,
        }
        let bundle: ClaimedBundle = resp.json().await?;

        let otk_b64 = bundle
            .one_time_key
            .map(|k| k.key)
            .or(bundle.fallback_key)
            .ok_or_else(|| anyhow!("peer bundle has neither one-time nor fallback key"))?;

        let identity_key =
            Curve25519PublicKey::from_base64(&bundle.identity_key).context("peer identity key")?;
        let one_time_key =
            Curve25519PublicKey::from_base64(&otk_b64).context("peer one-time key")?;

        let session = self.account.create_outbound_session(
            SessionConfig::version_1(),
            identity_key,
            one_time_key,
        );
        let session = session.context("create outbound session")?;

        self.peer_identity
            .insert(peer_device_id.to_string(), bundle.identity_key.clone());
        self.sessions.insert(bundle.identity_key.clone(), session);
        Ok(bundle.identity_key)
    }

    /// Open and authenticate a WebSocket to the relay (challenge-response over
    /// the transport key). Subsequent `recv` calls read from it.
    pub async fn connect(&mut self) -> Result<()> {
        let ws_url = self
            .base_url
            .replacen("http://", "ws://", 1)
            .replacen("https://", "wss://", 1)
            + "/v1/ws";
        let (mut ws, _) = connect_async(&ws_url).await.context("ws connect")?;

        // Challenge.
        let challenge = next_text(&mut ws)
            .await?
            .ok_or_else(|| anyhow!("ws closed"))?;
        let frame: ServerFrame = serde_json::from_str(&challenge)?;
        let nonce = match frame {
            ServerFrame::Challenge { nonce } => B64.decode(nonce)?,
            other => bail!("expected challenge, got {other:?}"),
        };
        let signature = B64.encode(
            self.signing_key
                .sign(&auth::ws_auth_payload(&nonce))
                .to_bytes(),
        );
        ws.send(WsMessage::Text(
            json!({ "type": "auth", "device_id": self.device_id, "signature": signature })
                .to_string()
                .into(),
        ))
        .await?;

        // Ready.
        let ready = next_text(&mut ws)
            .await?
            .ok_or_else(|| anyhow!("ws closed before ready"))?;
        match serde_json::from_str::<ServerFrame>(&ready)? {
            ServerFrame::Ready { .. } => {}
            ServerFrame::Error { reason } => bail!("ws auth rejected: {reason}"),
            other => bail!("expected ready, got {other:?}"),
        }
        self.ws = Some(ws);
        Ok(())
    }

    /// Wait for the next inbound message, decrypt it, and ACK it to the relay
    /// so the relay can delete it. Returns `None` if the connection closed.
    pub async fn recv(&mut self) -> Result<Option<Received>> {
        loop {
            let text = {
                let ws = self.ws.as_mut().context("not connected")?;
                match next_text(ws).await? {
                    Some(t) => t,
                    None => return Ok(None),
                }
            };
            match serde_json::from_str::<ServerFrame>(&text)? {
                ServerFrame::Message { id, ciphertext, .. } => {
                    let received = self.decrypt_frame(&id, &ciphertext)?;
                    // ACK regardless of decrypt outcome handled inside: only
                    // ACK on success so an undecryptable message is retried.
                    if let Some(r) = received {
                        self.ack(&id).await?;
                        return Ok(Some(r));
                    }
                }
                ServerFrame::Error { reason } => bail!("ws error: {reason}"),
                ServerFrame::Ready { .. } | ServerFrame::Challenge { .. } => {}
            }
        }
    }

    fn decrypt_frame(&mut self, _id: &str, ciphertext_b64: &str) -> Result<Option<Received>> {
        let wire = B64.decode(ciphertext_b64).context("decode ciphertext")?;
        let olm = wire::unpack(&wire).context("parse olm message")?;

        // Try existing sessions first (handles Normal messages and redelivered
        // pre-key messages for an already-established session).
        let sender_keys: Vec<String> = self.sessions.keys().cloned().collect();
        for key in sender_keys {
            if let Some(session) = self.sessions.get_mut(&key) {
                if let Ok(pt) = session.decrypt(&olm) {
                    return Ok(Some(Received {
                        sender_identity: key,
                        plaintext: String::from_utf8(pt).context("utf8 plaintext")?,
                        message_id: _id.to_string(),
                    }));
                }
            }
        }

        // No existing session decrypted it: a pre-key message must establish a
        // new inbound session. The sender's identity is embedded in the
        // message itself — the relay never told us who it is.
        if let OlmMessage::PreKey(prekey) = &olm {
            let sender_identity = prekey.identity_key();
            let result = self
                .account
                .create_inbound_session(SessionConfig::version_1(), sender_identity, prekey)
                .context("create inbound session")?;
            let key = sender_identity.to_base64();
            self.sessions.insert(key.clone(), result.session);
            return Ok(Some(Received {
                sender_identity: key,
                plaintext: String::from_utf8(result.plaintext).context("utf8 plaintext")?,
                message_id: _id.to_string(),
            }));
        }

        // A normal message we cannot decrypt (no matching session) — skip it.
        tracing::warn!("received undecryptable message; skipping");
        Ok(None)
    }

    async fn ack(&mut self, id: &str) -> Result<()> {
        let ws = self.ws.as_mut().context("not connected")?;
        ws.send(WsMessage::Text(
            json!({ "type": "ack", "id": id }).to_string().into(),
        ))
        .await?;
        Ok(())
    }

    /// Build a signed HTTP request (canonical string identical to the relay's).
    fn signed(&self, method: &str, path: &str, body: Vec<u8>) -> reqwest::RequestBuilder {
        let ts = unix_now();
        let canonical = auth::canonical_request(method, path, ts, &body);
        let signature = B64.encode(self.signing_key.sign(canonical.as_bytes()).to_bytes());
        let url = format!("{}{}", self.base_url, path);
        let builder = match method {
            "POST" => self.http.post(url),
            "GET" => self.http.get(url),
            other => panic!("unsupported method {other}"),
        };
        builder
            .header("content-type", "application/json")
            .header("x-device-id", &self.device_id)
            .header("x-timestamp", ts.to_string())
            .header("x-signature", signature)
            .body(body)
    }
}

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_secs()
}

async fn next_text(ws: &mut Ws) -> Result<Option<String>> {
    while let Some(msg) = ws.next().await {
        match msg? {
            WsMessage::Text(t) => return Ok(Some(t.to_string())),
            WsMessage::Close(_) => return Ok(None),
            _ => continue,
        }
    }
    Ok(None)
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerFrame {
    Challenge {
        nonce: String,
    },
    Ready {
        #[allow(dead_code)]
        pending: usize,
    },
    Message {
        id: String,
        ciphertext: String,
        #[allow(dead_code)]
        queued_at: u64,
        #[allow(dead_code)]
        expires_at: u64,
    },
    Error {
        reason: String,
    },
}
