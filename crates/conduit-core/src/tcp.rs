use crate::connection::{Connection, ConnectionKind, SignalState};
use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpServerConfig {
    /// Address to bind, e.g. "0.0.0.0" or "127.0.0.1"
    pub bind: String,
    pub port: u16,
}

pub async fn connect(config: TcpConfig) -> Result<Connection> {
    let addr = format!("{}:{}", config.host, config.port);
    let stream = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("failed to connect to {addr}"))?;
    info!(addr = %addr, "TCP connection opened");
    stream_to_connection(stream, ConnectionKind::TcpClient)
}

/// Binds a TCP listener and waits for exactly one incoming client.
/// Resolves once a client connects; the returned Connection represents
/// that single peer session.
pub async fn listen(config: TcpServerConfig) -> Result<Connection> {
    let addr = format!("{}:{}", config.bind, config.port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind TCP listener on {addr}"))?;

    info!(addr = %addr, "TCP server listening, waiting for client");

    let (stream, peer) = listener
        .accept()
        .await
        .with_context(|| format!("failed to accept on {addr}"))?;

    info!(addr = %addr, peer = %peer, "TCP client connected");

    stream_to_connection(stream, ConnectionKind::TcpServer)
}

fn stream_to_connection(stream: TcpStream, kind: ConnectionKind) -> Result<Connection> {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (inbound_tx, _)               = broadcast::channel(256);
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Bytes>(64);
    let (signals_tx, _)               = watch::channel(SignalState::default());

    let inbound_tx2 = inbound_tx.clone();

    let reader_task = tokio::spawn(async move {
        let mut buf = vec![0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => { let _ = inbound_tx2.send(Bytes::copy_from_slice(&buf[..n])); }
                Err(e) => { error!("TCP read error: {e}"); break; }
            }
        }
    });

    let writer_task = tokio::spawn(async move {
        while let Some(data) = outbound_rx.recv().await {
            if let Err(e) = writer.write_all(&data).await {
                error!("TCP write error: {e}"); break;
            }
        }
    });

    Ok(Connection::new(kind, inbound_tx, outbound_tx, signals_tx, None, vec![reader_task, writer_task]))
}
