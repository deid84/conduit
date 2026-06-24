use anyhow::Result;
use axum::Router;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    info!("headless gateway listening on 0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
