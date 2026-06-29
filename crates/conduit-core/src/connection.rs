use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for ConnectionId {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
}

impl std::str::FromStr for ConnectionId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(Self(s.parse()?)) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind { Serial, TcpClient, TcpServer, Udp }

/// Current state of RS-232 control/status lines.
/// Input lines (cd/dsr/cts/ri) are read from the port hardware.
/// Output lines (dtr/rts) are software-tracked — most ports cannot read them back.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SignalState {
    pub cd:  bool,   // Carrier Detect  — input
    pub dsr: bool,   // Data Set Ready  — input
    pub cts: bool,   // Clear To Send   — input
    pub ri:  bool,   // Ring Indicator  — input
    pub dtr: bool,   // Data Terminal Ready — output (software-tracked)
    pub rts: bool,   // Request To Send     — output (software-tracked)
}

#[derive(Debug)]
pub enum SignalCmd { SetDtr(bool), SetRts(bool) }

/// An active connection to a device or network endpoint.
///
/// - `inbound`:    subscribe to receive data from the device (broadcast, multi-consumer)
/// - `outbound`:   send raw bytes to the device
/// - `signals`:    watch the RS-232 signal state; for TCP/UDP the value never changes
/// - `signal_cmd`: send DTR/RTS control commands; None for TCP/UDP
pub struct Connection {
    pub id:         ConnectionId,
    pub kind:       ConnectionKind,
    pub inbound:    broadcast::Sender<Bytes>,
    pub outbound:   mpsc::Sender<Bytes>,
    pub signals:    watch::Sender<SignalState>,
    pub signal_cmd: Option<mpsc::Sender<SignalCmd>>,
    tasks:          Vec<tokio::task::JoinHandle<()>>,
}

impl Connection {
    pub(crate) fn new(
        kind:       ConnectionKind,
        inbound:    broadcast::Sender<Bytes>,
        outbound:   mpsc::Sender<Bytes>,
        signals:    watch::Sender<SignalState>,
        signal_cmd: Option<mpsc::Sender<SignalCmd>>,
        tasks:      Vec<tokio::task::JoinHandle<()>>,
    ) -> Self {
        Self { id: ConnectionId::new(), kind, inbound, outbound, signals, signal_cmd, tasks }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Bytes> { self.inbound.subscribe() }

    pub fn subscribe_signals(&self) -> watch::Receiver<SignalState> { self.signals.subscribe() }

    pub fn current_signals(&self) -> SignalState { self.signals.borrow().clone() }
}

impl Drop for Connection {
    fn drop(&mut self) { for t in &self.tasks { t.abort(); } }
}
