use axum::{extract::{Path, State}, http::StatusCode, Json};
use conduit_core::{connection::ConnectionId, SignalCmd, SignalState};
use serde::Deserialize;

use crate::state::AppState;

pub async fn get_signals(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SignalState>, StatusCode> {
    let id = id.parse::<ConnectionId>().map_err(|_| StatusCode::BAD_REQUEST)?;
    let conns = state.connections.read().await;
    let conn  = conns.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(conn.current_signals()))
}

#[derive(Deserialize)]
pub struct SetLineBody { pub value: bool }

pub async fn set_dtr(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<SetLineBody>,
) -> StatusCode {
    send_signal_cmd(id, state, SignalCmd::SetDtr(body.value)).await
}

pub async fn set_rts(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<SetLineBody>,
) -> StatusCode {
    send_signal_cmd(id, state, SignalCmd::SetRts(body.value)).await
}

async fn send_signal_cmd(id: String, state: AppState, cmd: SignalCmd) -> StatusCode {
    let id = match id.parse::<ConnectionId>() {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    let tx = {
        let conns = state.connections.read().await;
        match conns.get(&id) {
            Some(c) => c.signal_cmd.clone(),
            None    => return StatusCode::NOT_FOUND,
        }
    };
    match tx {
        Some(tx) => { let _ = tx.send(cmd).await; StatusCode::NO_CONTENT }
        None     => StatusCode::METHOD_NOT_ALLOWED,  // TCP/UDP have no control lines
    }
}
