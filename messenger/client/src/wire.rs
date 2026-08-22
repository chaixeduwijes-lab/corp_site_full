//! On-the-wire packing of an Olm message into the relay's opaque ciphertext
//! field.
//!
//! `vodozemac`'s [`OlmMessage`] splits into a numeric message type (0 = pre-key,
//! 1 = normal) and a ciphertext blob. We prepend the type as a single leading
//! byte so the recipient can reconstruct the exact `OlmMessage`. The relay
//! never interprets any of this — to it the whole thing is base64 of opaque
//! bytes.

use anyhow::{Context, Result};
use vodozemac::olm::OlmMessage;

/// Serialize an Olm message to `[type_byte] ++ ciphertext`.
pub fn pack(message: &OlmMessage) -> Vec<u8> {
    let (message_type, ciphertext) = message.to_parts();
    let mut out = Vec::with_capacity(1 + ciphertext.len());
    out.push(message_type as u8);
    out.extend_from_slice(&ciphertext);
    out
}

/// Reconstruct an Olm message from `[type_byte] ++ ciphertext`.
pub fn unpack(bytes: &[u8]) -> Result<OlmMessage> {
    let (&type_byte, ciphertext) = bytes.split_first().context("empty wire message")?;
    OlmMessage::from_parts(type_byte as usize, ciphertext)
        .map_err(|e| anyhow::anyhow!("decode olm message: {e}"))
}

/// Does this packed blob contain the given plaintext bytes? Used by tests to
/// assert the relay-visible bytes never contain the message text.
pub fn contains(wire: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && wire.windows(needle.len()).any(|w| w == needle)
}
