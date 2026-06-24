use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::state::AppState;

mod connections;
mod ports;
mod stream;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/ports", get(ports::list_ports))
        .route("/api/connections", get(connections::list).post(connections::open))
        .route("/api/connections/{id}", delete(connections::close))
        .route("/api/connections/{id}/send", post(connections::send))
        .route("/api/connections/{id}/stream", get(stream::handler))
        .with_state(state)
}
