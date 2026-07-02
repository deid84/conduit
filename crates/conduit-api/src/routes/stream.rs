use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use conduit_core::{connection::ConnectionId, modbus::Mode, SignalState};
use tokio::sync::{broadcast, watch};

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

    let (rx, sig_rx, modbus_mode) = {
        let conns = state.connections.read().await;
        match conns.get(&id) {
            Some(c) => (c.subscribe(), c.subscribe_signals(), c.modbus_mode),
            None    => return StatusCode::NOT_FOUND.into_response(),
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, rx, sig_rx, modbus_mode))
}

async fn handle_socket(
    mut socket:    WebSocket,
    mut rx:        broadcast::Receiver<bytes::Bytes>,
    mut sig_rx:    watch::Receiver<SignalState>,
    modbus_mode:   Option<Mode>,
) {
    let mut modbus_buf: Vec<u8> = Vec::new();

    loop {
        tokio::select! {
            data = rx.recv() => match data {
                Ok(bytes) => {
                    // Always forward raw bytes
                    if socket.send(Message::Binary(bytes.clone())).await.is_err() { break; }

                    // Modbus frame parsing overlay
                    if let Some(mode) = modbus_mode {
                        modbus_buf.extend_from_slice(&bytes);
                        loop {
                            if modbus_buf.is_empty() { break; }
                            match super::modbus::try_decode(mode, &modbus_buf) {
                                Some((frame, consumed)) => {
                                    modbus_buf.drain(..consumed);
                                    // Skip resync frames (unit_id=0, fc=0 placeholder)
                                    if frame.unit_id == 0 && matches!(frame.pdu, conduit_core::modbus::Pdu::Unknown { function_code: 0, .. }) {
                                        continue;
                                    }
                                    let json = serde_json::json!({
                                        "type": "modbus",
                                        "frame": frame,
                                    }).to_string();
                                    if socket.send(Message::Text(json.into())).await.is_err() { break; }
                                }
                                None => break, // need more data
                            }
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WebSocket client lagged by {n} messages");
                    modbus_buf.clear(); // buffer no longer coherent after a lag
                }
                Err(broadcast::error::RecvError::Closed) => {
                    let _ = socket.send(Message::Text(r#"{"type":"closed"}"#.into())).await;
                    break;
                }
            },

            result = sig_rx.changed() => {
                if result.is_err() { break; }
                let state = sig_rx.borrow_and_update().clone();
                let json  = serde_json::json!({ "type": "signals", "data": state }).to_string();
                if socket.send(Message::Text(json.into())).await.is_err() { break; }
            },

            msg = socket.recv() => { if msg.is_none() { break; } },
        }
    }
}
