pub mod connection;
pub mod modbus;
pub mod serial;
pub mod tcp;
pub mod udp;

pub use connection::{Connection, ConnectionId, ConnectionKind, SignalCmd, SignalState};
