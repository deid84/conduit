use axum::{
    routing::{delete, get, post},
    Router,
};
use std::path::PathBuf;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}};

use crate::state::AppState;

mod connections;
mod ports;
mod signals;
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
        .route("/api/connections/{id}/signals", get(signals::get_signals))
        .route("/api/connections/{id}/dtr", post(signals::set_dtr))
        .route("/api/connections/{id}/rts", post(signals::set_rts))
        .fallback_service(serve)
        .layer(CorsLayer::permissive())
        .with_state(state)
}
