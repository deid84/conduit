use axum::{
    routing::{delete, get, post},
    Router,
};
use std::path::PathBuf;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}};

use crate::state::AppState;

mod connections;
mod ports;
mod stream;

pub fn router(state: AppState, static_dir: PathBuf) -> Router {
    let serve = ServeDir::new(&static_dir)
        .fallback(ServeFile::new(static_dir.join("index.html")));

    Router::new()
        .route("/api/ports", get(ports::list_ports))
        .route("/api/connections", get(connections::list).post(connections::open))
        .route("/api/connections/{id}", delete(connections::close))
        .route("/api/connections/{id}/send", post(connections::send))
        .route("/api/connections/{id}/stream", get(stream::handler))
        .fallback_service(serve)
        .layer(CorsLayer::permissive())
        .with_state(state)
}
