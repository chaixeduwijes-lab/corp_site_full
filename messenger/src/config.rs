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
            max_queue_per_device: env_or("MSGR_MAX_QUEUE_PER_DEVICE", "512")
                .parse()
                .context("MSGR_MAX_QUEUE_PER_DEVICE must be an integer")?,
            max_devices: env_or("MSGR_MAX_DEVICES", "32")
                .parse()
                .context("MSGR_MAX_DEVICES must be an integer")?,
            max_body_bytes: env_or("MSGR_MAX_BODY_BYTES", "65536")
                .parse()
                .context("MSGR_MAX_BODY_BYTES must be an integer")?,
            tls,
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
