use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use bytes::Bytes;
use std::time::Duration;
use conduit_core::{
    connection::ConnectionId,
    modbus::{self, ascii, rtu, tcp as modbus_tcp, Mode, Pdu},
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::state::AppState;

// ── Set / clear modbus mode ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SetModeRequest {
    /// `null` to disable Modbus parsing.
    pub mode: Option<Mode>,
}

/// `POST /api/connections/{id}/modbus/mode`
///
/// Activates (or deactivates) Modbus frame parsing on the connection's WebSocket
/// stream. Takes effect immediately for new WebSocket subscribers; existing
/// subscribers must reconnect to pick up the change.
pub async fn set_mode(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<SetModeRequest>,
) -> StatusCode {
    let id = match id.parse::<ConnectionId>() {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    let mut conns = state.connections.write().await;
    match conns.get_mut(&id) {
        Some(c) => { c.modbus_mode = req.mode; StatusCode::NO_CONTENT }
        None    => StatusCode::NOT_FOUND,
    }
}

// ── Send a Modbus request ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SendRequest {
    pub unit_id:        u8,
    /// For Modbus TCP only; auto-incremented internally if omitted.
    pub transaction_id: Option<u16>,
    pub pdu:            Pdu,
}

/// `POST /api/connections/{id}/modbus/send`
///
/// Encodes `pdu` using the connection's current Modbus mode and sends the
/// raw frame bytes via the connection's outbound channel.
/// Returns `422` if no Modbus mode is set on the connection.
pub async fn send(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<SendRequest>,
) -> StatusCode {
    let id = match id.parse::<ConnectionId>() {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let (outbound, mode) = {
        let conns = state.connections.read().await;
        match conns.get(&id) {
            Some(c) => (c.outbound.clone(), c.modbus_mode),
            None    => return StatusCode::NOT_FOUND,
        }
    };

    let mode = match mode {
        Some(m) => m,
        None    => return StatusCode::UNPROCESSABLE_ENTITY,
    };

    let frame_bytes = encode_frame(mode, req.unit_id, req.transaction_id.unwrap_or(0), &req.pdu);

    match tokio::time::timeout(Duration::from_secs(5), outbound.send(Bytes::from(frame_bytes))).await {
        Ok(Ok(())) => StatusCode::NO_CONTENT,
        Ok(Err(_)) => StatusCode::GONE,
        Err(_)     => StatusCode::REQUEST_TIMEOUT,
    }
}

// ── Synchronous read (send + await response) ──────────────────────────────────

#[derive(Deserialize)]
pub struct ReadRequest {
    pub unit_id:        u8,
    pub transaction_id: Option<u16>,
    pub pdu:            Pdu,
    pub timeout_ms:     Option<u64>,
}

#[derive(Serialize)]
pub struct ReadResponse {
    pub pdu: Pdu,
}

/// `POST /api/connections/{id}/modbus/read`
///
/// Encodes `pdu`, sends it, then waits up to `timeout_ms` (default 1000) for
/// a decoded response frame on the inbound channel. Returns the response PDU
/// as JSON on success, `408` on timeout, `410` if the connection closed.
///
/// For RTU/ASCII the first valid non-request frame after sending is returned.
/// For TCP frames are additionally filtered by `transaction_id`.
/// Echo frames (ReadBits/ReadWords variants) are skipped automatically so
/// half-duplex RS-485 hardware echo does not confuse the matcher.
pub async fn read(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<ReadRequest>,
) -> Result<Json<ReadResponse>, StatusCode> {
    let id = match id.parse::<ConnectionId>() {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let (outbound, mut rx, mode) = {
        let conns = state.connections.read().await;
        match conns.get(&id) {
            Some(c) => (c.outbound.clone(), c.subscribe(), c.modbus_mode),
            None    => return Err(StatusCode::NOT_FOUND),
        }
    };

    let mode = match mode {
        Some(m) => m,
        None    => return Err(StatusCode::UNPROCESSABLE_ENTITY),
    };

    let timeout_ms = req.timeout_ms.unwrap_or(1000);
    let tx_id      = req.transaction_id.unwrap_or(0);
    let frame_bytes = encode_frame(mode, req.unit_id, tx_id, &req.pdu);

    // Send the request frame.
    match tokio::time::timeout(
        Duration::from_secs(5),
        outbound.send(Bytes::from(frame_bytes)),
    ).await {
        Ok(Ok(())) => {}
        Ok(Err(_)) => return Err(StatusCode::GONE),
        Err(_)     => return Err(StatusCode::REQUEST_TIMEOUT),
    }

    // Wait for a matching response.
    let result = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        async {
            let mut buf = Vec::<u8>::new();
            loop {
                match rx.recv().await {
                    Ok(chunk) => {
                        buf.extend_from_slice(&chunk);
                        // Try to drain all decodable frames from the buffer.
                        loop {
                            match try_decode(mode, &buf) {
                                Some((frame, n)) => {
                                    buf.drain(..n);
                                    // For TCP, filter by transaction_id.
                                    if matches!(mode, Mode::Tcp) {
                                        if frame.transaction_id != req.transaction_id {
                                            continue;
                                        }
                                    }
                                    // Skip echoed request frames (half-duplex RS-485).
                                    match &frame.pdu {
                                        Pdu::ReadBits  { .. } |
                                        Pdu::ReadWords { .. } => continue,
                                        _ => return Ok(frame.pdu),
                                    }
                                }
                                None => break, // need more data
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed)    => return Err(()),
                }
            }
        },
    ).await;

    match result {
        Ok(Ok(pdu)) => Ok(Json(ReadResponse { pdu })),
        Ok(Err(())) => Err(StatusCode::GONE),
        Err(_)      => Err(StatusCode::REQUEST_TIMEOUT),
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn encode_frame(mode: Mode, unit_id: u8, transaction_id: u16, pdu: &Pdu) -> Vec<u8> {
    match mode {
        Mode::Rtu   => rtu::encode(unit_id, pdu),
        Mode::Ascii => ascii::encode(unit_id, pdu),
        Mode::Tcp   => modbus_tcp::encode(transaction_id, unit_id, pdu),
    }
}

/// Try to decode one Modbus frame from `buf`.
///
/// Returns `Some((Frame, consumed_bytes))` on success, `None` when more data
/// is needed. On a parse error (bad CRC/LRC) one byte is consumed to trigger
/// resync on the next call.
pub(crate) fn try_decode(
    mode: Mode,
    buf:  &[u8],
) -> Option<(modbus::Frame, usize)> {
    match mode {
        Mode::Rtu => match rtu::decode(buf) {
            Ok(Some((f, n))) => Some((modbus::Frame {
                mode,
                unit_id:        f.unit_id,
                transaction_id: None,
                pdu:            f.decode_pdu(),
            }, n)),
            Ok(None)  => None,
            Err(_)    => Some((make_resync_frame(mode), 1)), // skip 1 byte
        },
        Mode::Ascii => match ascii::decode(buf) {
            Ok(Some((f, n))) => Some((modbus::Frame {
                mode,
                unit_id:        f.unit_id,
                transaction_id: None,
                pdu:            f.decode_pdu(),
            }, n)),
            Ok(None)  => None,
            Err(_)    => {
                // Skip to next ':' or 1 byte, whichever comes first
                let skip = buf.iter().skip(1).position(|&b| b == b':').map(|p| p + 1).unwrap_or(1);
                Some((make_resync_frame(mode), skip))
            }
        },
        Mode::Tcp => match modbus_tcp::decode(buf) {
            Ok(Some((f, n))) => Some((modbus::Frame {
                mode,
                unit_id:        f.unit_id,
                transaction_id: Some(f.transaction_id),
                pdu:            f.decode_pdu(),
            }, n)),
            Ok(None)  => None,
            Err(_)    => Some((make_resync_frame(mode), 1)),
        },
    }
}

fn make_resync_frame(mode: Mode) -> modbus::Frame {
    modbus::Frame {
        mode,
        unit_id:        0,
        transaction_id: None,
        pdu:            Pdu::Unknown { function_code: 0, data: vec![] },
    }
}
