use axum::{http::StatusCode, Json};
use conduit_core::serial;

pub async fn list_ports() -> Result<Json<Vec<serial::PortInfo>>, StatusCode> {
    serial::list_ports().map(Json).map_err(|e| {
        tracing::error!("list_ports: {e:#}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
