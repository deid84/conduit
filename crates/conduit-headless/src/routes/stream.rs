use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use conduit_core::connection::ConnectionId;
use tokio::sync::broadcast;

use crate::state::AppState;

pub async fn handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let id = match id.parse::<ConnectionId>() {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let rx = {
        let conns = state.connections.read().await;
        match conns.get(&id) {
            Some(c) => c.subscribe(),
            None => return StatusCode::NOT_FOUND.into_response(),
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, rx))
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<bytes::Bytes>) {
    loop {
        tokio::select! {
            data = rx.recv() => {
                match data {
                    Ok(bytes) => {
                        if socket.send(Message::Binary(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket client lagged by {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        let _ = socket.send(Message::Text(r#"{"type":"closed"}"#.into())).await;
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                if msg.is_none() {
                    break;
                }
            }
        }
    }
}
