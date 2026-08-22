use std::time::Duration;

use axum_server::tls_rustls::RustlsConfig;
use axum_server::Handle;
use tracing_subscriber::EnvFilter;

use messenger_relay::build_router;
use messenger_relay::config::Config;
use messenger_relay::registry::DeviceRegistry;
use messenger_relay::state::{unix_now, AppState, SharedState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Privacy-preserving defaults: lifecycle events only. Message-level
    // events are logged at debug and stay off in production.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;
    let registry = DeviceRegistry::open(&config.db_path)?;
    tracing::info!(
        devices = registry.count(),
        ttl_secs = config.message_ttl.as_secs(),
        registration_open = config.invite_token.is_some(),
        "starting messenger-relay"
    );

    let state = AppState::new(config.clone(), registry);
    tokio::spawn(sweeper(state.clone()));

    let app = build_router(state);
    let handle = Handle::new();
    tokio::spawn(shutdown_on_signal(handle.clone()));

    match &config.tls {
        Some(tls) => {
            let rustls = RustlsConfig::from_pem_file(&tls.cert_path, &tls.key_path).await?;
            tracing::info!(addr = %config.bind_addr, "listening (tls)");
            axum_server::bind_rustls(config.bind_addr, rustls)
                .handle(handle)
                .serve(app.into_make_service())
                .await?;
        }
        None => {
            tracing::info!(addr = %config.bind_addr, "listening (plaintext — terminate TLS in front or set MSGR_TLS_CERT/MSGR_TLS_KEY)");
            axum_server::bind(config.bind_addr)
                .handle(handle)
                .serve(app.into_make_service())
                .await?;
        }
    }

    tracing::info!("shut down");
    Ok(())
}

/// Periodically enforces the message TTL and prunes the auth replay cache.
async fn sweeper(state: SharedState) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        let now = unix_now();
        let dropped = state.queue.sweep_expired(now);
        if dropped > 0 {
            tracing::debug!(dropped, "expired messages dropped");
        }
        state.replay.sweep(now);
    }
}

async fn shutdown_on_signal(handle: Handle) {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
    tracing::info!("shutdown signal received");
    handle.graceful_shutdown(Some(Duration::from_secs(10)));
}
