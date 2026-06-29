use crate::connection::{Connection, ConnectionKind, SignalCmd, SignalState};
use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{error, info};

// Box<dyn SerialPort> is not Send because the trait doesn't require it,
// but the concrete platform implementations (TTYPort / COMPort) hold a plain
// file descriptor / HANDLE that is safe to move between threads.
struct SendPort(Box<dyn serialport::SerialPort>);
unsafe impl Send for SendPort {}

impl Read  for SendPort { fn read (&mut self, buf: &mut [u8]) -> std::io::Result<usize> { self.0.read(buf)  } }
impl Write for SendPort { fn write(&mut self, buf: &[u8])     -> std::io::Result<usize> { self.0.write(buf) }
                          fn flush(&mut self)                  -> std::io::Result<()>    { self.0.flush()   } }

impl std::ops::Deref    for SendPort { type Target = dyn serialport::SerialPort; fn deref    (&self)     -> &Self::Target { &*self.0 } }
impl std::ops::DerefMut for SendPort {                                            fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.0 } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
    #[serde(default)] pub data_bits:    DataBits,
    #[serde(default)] pub stop_bits:    StopBits,
    #[serde(default)] pub parity:       Parity,
    #[serde(default)] pub flow_control: FlowControl,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataBits { Five, Six, Seven, #[default] Eight }

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopBits { #[default] One, Two }

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parity { #[default] None, Odd, Even }

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowControl { #[default] None, Software, Hardware }

impl From<DataBits>    for serialport::DataBits    { fn from(v: DataBits)    -> Self { match v { DataBits::Five => Self::Five, DataBits::Six => Self::Six, DataBits::Seven => Self::Seven, DataBits::Eight => Self::Eight } } }
impl From<StopBits>    for serialport::StopBits    { fn from(v: StopBits)    -> Self { match v { StopBits::One => Self::One, StopBits::Two => Self::Two } } }
impl From<Parity>      for serialport::Parity      { fn from(v: Parity)      -> Self { match v { Parity::None => Self::None, Parity::Odd => Self::Odd, Parity::Even => Self::Even } } }
impl From<FlowControl> for serialport::FlowControl { fn from(v: FlowControl) -> Self { match v { FlowControl::None => Self::None, FlowControl::Software => Self::Software, FlowControl::Hardware => Self::Hardware } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo { pub name: String }

pub fn list_ports() -> Result<Vec<PortInfo>> {
    let ports = serialport::available_ports().context("failed to enumerate serial ports")?;
    Ok(ports.into_iter().map(|p| PortInfo { name: p.port_name }).collect())
}

pub fn open(config: SerialConfig) -> Result<Connection> {
    // Use sync serialport (no tokio_serial) so we can use try_recv() for writes
    // and signal commands without needing to clone the port handle.
    // A 5 ms read timeout lets the loop stay responsive for outbound data / signals.
    let port = serialport::new(&config.port, config.baud_rate)
        .data_bits(config.data_bits.into())
        .stop_bits(config.stop_bits.into())
        .parity(config.parity.into())
        .flow_control(config.flow_control.into())
        .timeout(Duration::from_millis(5))
        .open()
        .with_context(|| format!("failed to open serial port {}", config.port))?;

    let (inbound_tx, _)                    = broadcast::channel(256);
    let (outbound_tx, mut outbound_rx)     = mpsc::channel::<Bytes>(64);
    let (signals_tx, _)                    = watch::channel(SignalState::default());
    let (signal_cmd_tx, mut signal_cmd_rx) = mpsc::channel::<SignalCmd>(8);

    let inbound_tx2 = inbound_tx.clone();
    let signals_tx2 = signals_tx.clone();
    let port_name   = config.port.clone();

    // One blocking thread owns the port handle; uses try_recv() for zero-copy
    // channel polling without needing a second fd or async runtime inside.
    let task = tokio::task::spawn_blocking(move || {
        let mut port     = SendPort(port);
        let mut buf      = vec![0u8; 4096];
        let sig_interval = Duration::from_millis(250);
        let mut last_sig = Instant::now();
        let mut cur_dtr  = false;
        let mut cur_rts  = false;

        loop {
            // Drain outbound data queue.
            while let Ok(data) = outbound_rx.try_recv() {
                if port.write_all(&data).is_err() { return; }
            }

            // Drain signal command queue.
            while let Ok(cmd) = signal_cmd_rx.try_recv() {
                match cmd {
                    SignalCmd::SetDtr(v) => { if port.write_data_terminal_ready(v).is_ok() { cur_dtr = v; } }
                    SignalCmd::SetRts(v) => { if port.write_request_to_send(v).is_ok()     { cur_rts = v; } }
                }
            }

            // Poll modem status lines every 250 ms; push WS frame only on change.
            if last_sig.elapsed() >= sig_interval {
                last_sig = Instant::now();
                let new = SignalState {
                    cd:  port.read_carrier_detect().unwrap_or(false),
                    dsr: port.read_data_set_ready().unwrap_or(false),
                    cts: port.read_clear_to_send().unwrap_or(false),
                    ri:  port.read_ring_indicator().unwrap_or(false),
                    dtr: cur_dtr,
                    rts: cur_rts,
                };
                signals_tx2.send_if_modified(|s| {
                    if *s != new { *s = new; true } else { false }
                });
            }

            // Blocking read (up to 5 ms timeout set at open).
            match port.read(&mut buf) {
                Ok(0)  => break,
                Ok(n)  => { let _ = inbound_tx2.send(Bytes::copy_from_slice(&buf[..n])); }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => { error!(port = %port_name, "read error: {e}"); break; }
            }
        }
    });

    info!(port = %config.port, baud = config.baud_rate, "serial connection opened");

    Ok(Connection::new(
        ConnectionKind::Serial,
        inbound_tx,
        outbound_tx,
        signals_tx,
        Some(signal_cmd_tx),
        vec![task],
    ))
}
