use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for ConnectionId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    Serial,
    TcpClient,
    TcpServer,
    Udp,
}

/// An active connection to a device or network endpoint.
///
/// - `inbound`: subscribe to receive data coming **from** the device.
///   Uses broadcast so multiple WebSocket clients can all subscribe simultaneously.
/// - `outbound`: send data **to** the device.
pub struct Connection {
    pub id: ConnectionId,
    pub kind: ConnectionKind,
    pub inbound: broadcast::Sender<Bytes>,
    pub outbound: mpsc::Sender<Bytes>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Connection {
    pub(crate) fn new(
        kind: ConnectionKind,
        inbound: broadcast::Sender<Bytes>,
        outbound: mpsc::Sender<Bytes>,
        tasks: Vec<tokio::task::JoinHandle<()>>,
    ) -> Self {
        Self {
            id: ConnectionId::new(),
            kind,
            inbound,
            outbound,
            tasks,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Bytes> {
        self.inbound.subscribe()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}
