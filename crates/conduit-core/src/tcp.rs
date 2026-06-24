use crate::connection::{Connection, ConnectionKind};
use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConfig {
    pub host: String,
    pub port: u16,
}

pub async fn connect(config: TcpConfig) -> Result<Connection> {
    let addr = format!("{}:{}", config.host, config.port);
    let stream = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("failed to connect to {addr}"))?;

    let (mut reader, mut writer) = tokio::io::split(stream);
    let (inbound_tx, _) = broadcast::channel(256);
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Bytes>(64);

    let inbound_tx_clone = inbound_tx.clone();
    let addr_clone = addr.clone();

    let reader_task = tokio::spawn(async move {
        let mut buf = vec![0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let _ = inbound_tx_clone.send(Bytes::copy_from_slice(&buf[..n]));
                }
                Err(e) => {
                    error!(addr = %addr_clone, "read error: {e}");
                    break;
                }
            }
        }
    });

    let writer_task = tokio::spawn(async move {
        while let Some(data) = outbound_rx.recv().await {
            if let Err(e) = writer.write_all(&data).await {
                error!(addr = %addr, "write error: {e}");
                break;
            }
        }
    });

    info!(addr = %config.host, port = config.port, "TCP connection opened");

    Ok(Connection::new(
        ConnectionKind::TcpClient,
        inbound_tx,
        outbound_tx,
        vec![reader_task, writer_task],
    ))
}
