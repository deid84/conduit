use anyhow::Result;
use conduit_headless::{routes, state};
use std::path::PathBuf;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present (errors silently ignored — file is optional)
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let static_dir: PathBuf = std::env::var("CONDUIT_STATIC_DIR")
        .unwrap_or_else(|_| "frontend/dist".to_string())
        .into();

    info!("serving frontend from {}", static_dir.display());

    let state = state::AppState::new();
    let app = routes::router(state, static_dir);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("headless gateway listening on 0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
