use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;

/// Hard ceiling for the message TTL. The relay must never keep a message
/// longer than 20 minutes, whatever the environment says.
pub const MAX_TTL: Duration = Duration::from_secs(20 * 60);
pub const MIN_TTL: Duration = Duration::from_secs(10);

/// Accepted clock skew between client and server for signed requests.
pub const AUTH_WINDOW_SECS: u64 = 300;

#[derive(Clone, Debug)]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub db_path: PathBuf,
    /// Pre-shared invite token gating registration. `None` disables
    /// registration entirely (existing devices keep working).
    pub invite_token: Option<String>,
    pub message_ttl: Duration,
    pub max_queue_per_device: usize,
    pub max_devices: usize,
    pub max_body_bytes: usize,
    /// Relay-wide ceiling on the total bytes of queued ciphertext held in RAM.
    pub max_total_queue_bytes: usize,
    /// Lock memory and disable core dumps at startup (F1). Default on (unix).
    pub memory_hardening: bool,
    pub tls: Option<TlsConfig>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr = env_or("MSGR_BIND_ADDR", "127.0.0.1:8080")
            .parse()
            .context("MSGR_BIND_ADDR must be a socket address like 0.0.0.0:443")?;

        let ttl_secs: u64 = env_or("MSGR_MESSAGE_TTL_SECS", "1200")
            .parse()
            .context("MSGR_MESSAGE_TTL_SECS must be an integer")?;
        let message_ttl = Duration::from_secs(ttl_secs).clamp(MIN_TTL, MAX_TTL);

        let tls = match (
            std::env::var("MSGR_TLS_CERT"),
            std::env::var("MSGR_TLS_KEY"),
        ) {
            (Ok(cert), Ok(key)) => Some(TlsConfig {
                cert_path: cert.into(),
                key_path: key.into(),
            }),
            (Err(_), Err(_)) => None,
            _ => anyhow::bail!("MSGR_TLS_CERT and MSGR_TLS_KEY must be set together"),
        };

        Ok(Self {
            bind_addr,
            db_path: env_or("MSGR_DB_PATH", "devices.db").into(),
            invite_token: std::env::var("MSGR_INVITE_TOKEN")
                .ok()
                .filter(|t| !t.is_empty()),
            message_ttl,
            // Defaults sized for a ~1.5 GB privacy VPS (Njalla/FlokiNET, see
            // docs). Worst case per-device queue: 16 KiB * 64 = 1 MiB; the
            // global budget below caps the whole relay well under host RAM.
            max_queue_per_device: env_or("MSGR_MAX_QUEUE_PER_DEVICE", "64")
                .parse()
                .context("MSGR_MAX_QUEUE_PER_DEVICE must be an integer")?,
            max_devices: env_or("MSGR_MAX_DEVICES", "16")
                .parse()
                .context("MSGR_MAX_DEVICES must be an integer")?,
            max_body_bytes: env_or("MSGR_MAX_BODY_BYTES", "16384")
                .parse()
                .context("MSGR_MAX_BODY_BYTES must be an integer")?,
            // 64 MiB default: comfortably below the RAM of the smallest
            // recommended VPS even with the OS and process overhead.
            max_total_queue_bytes: env_or("MSGR_MAX_TOTAL_QUEUE_BYTES", "67108864")
                .parse()
                .context("MSGR_MAX_TOTAL_QUEUE_BYTES must be an integer")?,
            memory_hardening: env_bool("MSGR_MEMORY_HARDENING", true)?,
            tls,
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> anyhow::Result<bool> {
    match std::env::var(key) {
        Err(_) => Ok(default),
        Ok(v) => match v.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => anyhow::bail!("{key} must be a boolean (true/false)"),
        },
    }
}
