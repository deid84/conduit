use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use conduit_core::{connection::ConnectionId, serial, tcp, udp, ConnectionKind};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
#[serde(tag = "type", content = "config", rename_all = "snake_case")]
pub enum OpenRequest {
    Serial(serial::SerialConfig),
    Tcp(tcp::TcpConfig),
    TcpServer(tcp::TcpServerConfig),
    Udp(udp::UdpConfig),
}

#[derive(Serialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub kind: ConnectionKind,
}

pub async fn list(State(state): State<AppState>) -> Json<Vec<ConnectionInfo>> {
    let conns = state.connections.read().await;
    Json(
        conns
            .iter()
            .map(|(id, c)| ConnectionInfo {
                id: id.to_string(),
                kind: c.kind,
            })
            .collect(),
    )
}

pub async fn open(
    State(state): State<AppState>,
    Json(req): Json<OpenRequest>,
) -> Result<(StatusCode, Json<ConnectionInfo>), (StatusCode, String)> {
    let conn = match req {
        OpenRequest::Serial(cfg)    => serial::open(cfg),
        OpenRequest::Tcp(cfg)       => tcp::connect(cfg).await,
        OpenRequest::TcpServer(cfg) => tcp::listen(cfg).await,
        OpenRequest::Udp(cfg)       => udp::bind(cfg).await,
    }
    .map_err(|e| {
        tracing::warn!("open connection failed: {e:#}");
        (StatusCode::UNPROCESSABLE_ENTITY, format!("{e:#}"))
    })?;

    let info = ConnectionInfo {
        id: conn.id.to_string(),
        kind: conn.kind,
    };
    state.connections.write().await.insert(conn.id.clone(), conn);

    Ok((StatusCode::CREATED, Json(info)))
}

pub async fn close(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> StatusCode {
    match id.parse::<ConnectionId>() {
        Err(_) => StatusCode::BAD_REQUEST,
        Ok(id) => {
            if state.connections.write().await.remove(&id).is_some() {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::NOT_FOUND
            }
        }
    }
}

pub async fn send(
    Path(id): Path<String>,
    State(state): State<AppState>,
    body: Bytes,
) -> StatusCode {
    let id = match id.parse::<ConnectionId>() {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let sender = {
        let conns = state.connections.read().await;
        match conns.get(&id) {
            Some(c) => c.outbound.clone(),
            None => return StatusCode::NOT_FOUND,
        }
    };

    if sender.send(body).await.is_ok() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::GONE
    }
}
