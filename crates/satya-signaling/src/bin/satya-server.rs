//! Binary entrypoint for the Satya WebRTC Signaling & Apache Iggy Event Streaming Server.
#![deny(missing_docs)]

use satya_signaling::{SignalingAppState, build_router};
use satya_storage::{AiGmConfig, StorageConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,satya_signaling=debug,satya_storage=debug".into()),
        )
        .init();

    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let storage_config_path = base_dir.join("config").join("storage.toml");
    let ai_config_path = base_dir.join("config").join("ai.toml");
    let mut storage_cfg = if storage_config_path.exists() {
        StorageConfig::load_from_file(&storage_config_path)?
    } else {
        StorageConfig::default()
    };
    if ai_config_path.exists() {
        storage_cfg.ai_gm = AiGmConfig::load_from_file(&ai_config_path)?;
    }

    let state = SignalingAppState::new(storage_cfg, &base_dir).await?;
    let app = build_router(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Satya WebRTC & Apache Iggy Event Server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
