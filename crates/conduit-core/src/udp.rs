use crate::connection::{Connection, ConnectionKind, SignalState};
use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpConfig {
    /// Local address to bind (e.g. "0.0.0.0:0" for ephemeral port).
    pub bind: String,
    /// Remote address to send to (e.g. "192.168.1.10:5000").
    pub remote: String,
}

pub async fn bind(config: UdpConfig) -> Result<Connection> {
    let socket = UdpSocket::bind(&config.bind)
        .await
        .with_context(|| format!("failed to bind UDP socket to {}", config.bind))?;

    socket
        .connect(&config.remote)
        .await
        .with_context(|| format!("failed to set UDP remote to {}", config.remote))?;

    let socket = std::sync::Arc::new(socket);
    let (inbound_tx, _) = broadcast::channel(256);
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Bytes>(64);
    let (signals_tx, _) = watch::channel(SignalState::default());

    let inbound_tx_clone = inbound_tx.clone();
    let reader_socket = socket.clone();
    let remote_clone = config.remote.clone();

    let reader_task = tokio::spawn(async move {
        let mut buf = vec![0u8; 65535];
        loop {
            match reader_socket.recv(&mut buf).await {
                Ok(n) => {
                    let _ = inbound_tx_clone.send(Bytes::copy_from_slice(&buf[..n]));
                }
                Err(e) => {
                    error!(remote = %remote_clone, "UDP recv error: {e}");
                    break;
                }
            }
        }
    });

    let remote = config.remote.clone();
    let bind = config.bind.clone();

    let writer_task = tokio::spawn(async move {
        while let Some(data) = outbound_rx.recv().await {
            if let Err(e) = socket.send(&data).await {
                error!(remote = %remote, "UDP send error: {e}");
                break;
            }
        }
    });

    info!(bind = %bind, remote = %config.remote, "UDP socket bound");

    Ok(Connection::new(
        ConnectionKind::Udp,
        inbound_tx,
        outbound_tx,
        signals_tx,
        None,
        vec![reader_task, writer_task],
    ))
}
