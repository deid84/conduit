#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let serve = std::env::args().any(|a| a == "-s" || a == "--serve");
    if serve {
        let rt = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
        if let Err(e) = rt.block_on(serve_mode()) {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    } else {
        conduit_tauri_lib::run();
    }
}

async fn serve_mode() -> anyhow::Result<()> {
    use conduit_api::{routes, state};
    use std::path::PathBuf;

    tracing_subscriber::fmt::init();

    let static_dir: PathBuf = std::env::var("CONDUIT_STATIC_DIR")
        .unwrap_or_else(|_| "frontend/dist".to_string())
        .into();

    let port: u16 = std::env::var("CONDUIT_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let bind = std::env::var("CONDUIT_BIND")
        .unwrap_or_else(|_| format!("0.0.0.0:{port}"));

    tracing::info!("serving frontend from {}", static_dir.display());

    let st = state::AppState::new();
    let app = routes::router(st, static_dir);

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("serving on http://{bind}");
    axum::serve(listener, app).await?;

    Ok(())
}
