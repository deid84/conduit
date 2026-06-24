use crate::connection::{Connection, ConnectionKind};
use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc};
use tokio_serial::SerialPortBuilderExt;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
    #[serde(default)]
    pub data_bits: DataBits,
    #[serde(default)]
    pub stop_bits: StopBits,
    #[serde(default)]
    pub parity: Parity,
    #[serde(default)]
    pub flow_control: FlowControl,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataBits {
    Five,
    Six,
    Seven,
    #[default]
    Eight,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopBits {
    #[default]
    One,
    Two,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parity {
    #[default]
    None,
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowControl {
    #[default]
    None,
    Software,
    Hardware,
}

impl From<DataBits> for serialport::DataBits {
    fn from(v: DataBits) -> Self {
        match v {
            DataBits::Five => Self::Five,
            DataBits::Six => Self::Six,
            DataBits::Seven => Self::Seven,
            DataBits::Eight => Self::Eight,
        }
    }
}

impl From<StopBits> for serialport::StopBits {
    fn from(v: StopBits) -> Self {
        match v {
            StopBits::One => Self::One,
            StopBits::Two => Self::Two,
        }
    }
}

impl From<Parity> for serialport::Parity {
    fn from(v: Parity) -> Self {
        match v {
            Parity::None => Self::None,
            Parity::Odd => Self::Odd,
            Parity::Even => Self::Even,
        }
    }
}

impl From<FlowControl> for serialport::FlowControl {
    fn from(v: FlowControl) -> Self {
        match v {
            FlowControl::None => Self::None,
            FlowControl::Software => Self::Software,
            FlowControl::Hardware => Self::Hardware,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
}

pub fn list_ports() -> Result<Vec<PortInfo>> {
    let ports = serialport::available_ports().context("failed to enumerate serial ports")?;
    Ok(ports.into_iter().map(|p| PortInfo { name: p.port_name }).collect())
}

pub fn open(config: SerialConfig) -> Result<Connection> {
    let stream = serialport::new(&config.port, config.baud_rate)
        .data_bits(config.data_bits.into())
        .stop_bits(config.stop_bits.into())
        .parity(config.parity.into())
        .flow_control(config.flow_control.into())
        .open_native_async()
        .with_context(|| format!("failed to open serial port {}", config.port))?;

    let (mut reader, mut writer) = tokio::io::split(stream);
    let (inbound_tx, _) = broadcast::channel(256);
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Bytes>(64);

    let inbound_tx_clone = inbound_tx.clone();
    let port_name = config.port.clone();

    let reader_task = tokio::spawn(async move {
        let mut buf = vec![0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let _ = inbound_tx_clone.send(Bytes::copy_from_slice(&buf[..n]));
                }
                Err(e) => {
                    error!(port = %port_name, "read error: {e}");
                    break;
                }
            }
        }
    });

    let port_name = config.port.clone();
    let writer_task = tokio::spawn(async move {
        while let Some(data) = outbound_rx.recv().await {
            if let Err(e) = writer.write_all(&data).await {
                error!(port = %port_name, "write error: {e}");
                break;
            }
        }
    });

    info!(port = %config.port, baud = config.baud_rate, "serial connection opened");

    Ok(Connection::new(
        ConnectionKind::Serial,
        inbound_tx,
        outbound_tx,
        vec![reader_task, writer_task],
    ))
}
