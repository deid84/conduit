pub mod ascii;
pub mod pdu;
pub mod rtu;
pub mod tcp;

pub use pdu::{ExceptionCode, FunctionCode, Pdu};

use std::fmt;

/// Modbus transmission mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Rtu,
    Ascii,
    Tcp,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Rtu   => write!(f, "RTU"),
            Mode::Ascii => write!(f, "ASCII"),
            Mode::Tcp   => write!(f, "TCP"),
        }
    }
}

/// A fully decoded Modbus frame, transport-agnostic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    pub mode:           Mode,
    pub unit_id:        u8,
    /// Set only for TCP frames.
    pub transaction_id: Option<u16>,
    pub pdu:            Pdu,
}

/// Modbus parsing error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Buffer is too short; caller should accumulate more bytes.
    NeedMoreData,
    /// RTU CRC does not match.
    CrcMismatch { expected: u16, got: u16 },
    /// ASCII LRC does not match.
    LrcMismatch { expected: u8, got: u8 },
    /// ASCII frame structure or hex encoding is invalid.
    InvalidAscii(&'static str),
    /// TCP MBAP length field is inconsistent.
    InvalidLength,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NeedMoreData =>
                write!(f, "need more data"),
            Error::CrcMismatch { expected, got } =>
                write!(f, "CRC mismatch: expected {expected:#06x}, got {got:#06x}"),
            Error::LrcMismatch { expected, got } =>
                write!(f, "LRC mismatch: expected {expected:#04x}, got {got:#04x}"),
            Error::InvalidAscii(reason) =>
                write!(f, "invalid ASCII frame: {reason}"),
            Error::InvalidLength =>
                write!(f, "invalid frame length"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
