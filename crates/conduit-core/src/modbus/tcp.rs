use super::{Error, Result};
use super::pdu::{decode_pdu, Pdu};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Length of the MBAP header: TxID(2) + ProtoID(2) + Length(2) + UnitID(1) = 7 bytes.
pub const MBAP_LEN: usize = 7;

// ── Frame type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpFrame {
    pub transaction_id: u16,
    pub unit_id:        u8,
    pub function_code:  u8,
    /// Bytes after the function code (excludes MBAP header and FC byte).
    pub data:           Vec<u8>,
}

impl TcpFrame {
    pub fn decode_pdu(&self) -> Pdu {
        decode_pdu(self.function_code, &self.data)
    }
}

// ── Decoder ───────────────────────────────────────────────────────────────────

/// Attempt to decode one Modbus TCP frame from `buf`.
///
/// MBAP wire format:
/// ```text
/// [TxID 2B] [ProtoID 2B = 0x0000] [Length 2B] [UnitID 1B] [FC 1B] [Data…]
/// ```
/// `Length` counts bytes from `UnitID` to end of frame (minimum 2: UnitID + FC).
///
/// Returns:
/// - `Ok(Some((frame, consumed)))` — complete frame decoded.
/// - `Ok(None)` — buffer too short; accumulate more bytes.
/// - `Err(Error::InvalidLength)` — protocol ID ≠ 0 or length field < 2.
pub fn decode(buf: &[u8]) -> Result<Option<(TcpFrame, usize)>> {
    // Need at least the fixed 6-byte prefix to read the Length field
    if buf.len() < 6 {
        return Ok(None);
    }

    let transaction_id = u16::from_be_bytes([buf[0], buf[1]]);
    let protocol_id    = u16::from_be_bytes([buf[2], buf[3]]);
    let length         = u16::from_be_bytes([buf[4], buf[5]]) as usize;

    if protocol_id != 0 {
        return Err(Error::InvalidLength); // not Modbus TCP
    }
    if length < 2 {
        return Err(Error::InvalidLength); // must contain at least UnitID + FC
    }

    let total = 6 + length;
    if buf.len() < total {
        return Ok(None);
    }

    let unit_id       = buf[6];
    let function_code = buf[7];
    let data          = buf[8..total].to_vec();

    Ok(Some((TcpFrame { transaction_id, unit_id, function_code, data }, total)))
}

// ── Encoder ───────────────────────────────────────────────────────────────────

/// Encode a Modbus TCP frame.
///
/// `transaction_id` should be incremented by the caller for each new request
/// so that responses can be matched.
pub fn encode(transaction_id: u16, unit_id: u8, pdu: &Pdu) -> Vec<u8> {
    let pdu_bytes = pdu.encode();
    // Length = UnitID(1) + PDU bytes
    let length = (1 + pdu_bytes.len()) as u16;

    let mut frame = Vec::with_capacity(6 + 1 + pdu_bytes.len());
    frame.extend_from_slice(&transaction_id.to_be_bytes());
    frame.extend_from_slice(&0u16.to_be_bytes()); // Protocol ID = 0
    frame.extend_from_slice(&length.to_be_bytes());
    frame.push(unit_id);
    frame.extend_from_slice(&pdu_bytes);
    frame
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modbus::pdu::Pdu;

    // Known-good vector:
    // Read Holding Registers, TxID=0x0001, unit=1, start=0x006B, count=3
    // 00 01 00 00 00 06 01 03 00 6B 00 03
    const READ_REGS_REQ: [u8; 12] = [
        0x00, 0x01, // TxID
        0x00, 0x00, // ProtoID
        0x00, 0x06, // Length = 6 (unit + FC + 4 data bytes)
        0x01,       // UnitID
        0x03,       // FC
        0x00, 0x6B, // start address
        0x00, 0x03, // quantity
    ];

    #[test]
    fn decode_known_frame() {
        let (frame, consumed) = decode(&READ_REGS_REQ).unwrap().unwrap();
        assert_eq!(consumed, 12);
        assert_eq!(frame.transaction_id, 1);
        assert_eq!(frame.unit_id, 1);
        assert_eq!(frame.function_code, 0x03);
        assert_eq!(frame.data, [0x00, 0x6B, 0x00, 0x03]);
    }

    #[test]
    fn decode_too_short_returns_none() {
        assert!(decode(&READ_REGS_REQ[..5]).unwrap().is_none());
        assert!(decode(&READ_REGS_REQ[..10]).unwrap().is_none());
    }

    #[test]
    fn decode_wrong_protocol_id() {
        let mut bad = READ_REGS_REQ;
        bad[2] = 0x00;
        bad[3] = 0x01; // ProtoID = 1 instead of 0
        assert!(matches!(decode(&bad), Err(Error::InvalidLength)));
    }

    #[test]
    fn encode_produces_known_frame() {
        let pdu = Pdu::ReadWords { fc: 0x03, start: 0x006B, count: 3 };
        let frame = encode(1, 1, &pdu);
        assert_eq!(frame.as_slice(), &READ_REGS_REQ);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let pdu = Pdu::WriteSingleRegister { address: 0x0010, value: 0xABCD };
        let raw = encode(42, 3, &pdu);
        let (frame, consumed) = decode(&raw).unwrap().unwrap();
        assert_eq!(consumed, raw.len());
        assert_eq!(frame.transaction_id, 42);
        assert_eq!(frame.unit_id, 3);
        match frame.decode_pdu() {
            Pdu::WriteSingleRegister { address: 0x0010, value: 0xABCD } => {}
            other => panic!("unexpected PDU: {other:?}"),
        }
    }

    #[test]
    fn encode_decode_write_multiple_registers() {
        let values = vec![0x000A, 0x0102];
        let pdu = Pdu::WriteMultipleRegisters {
            address: 0x0001,
            count:   2,
            values:  values.clone(),
        };
        let raw = encode(1, 1, &pdu);
        let (frame, _) = decode(&raw).unwrap().unwrap();
        match frame.decode_pdu() {
            Pdu::WriteMultipleRegisters { address: 0x0001, count: 2, values: v } => {
                assert_eq!(v, values);
            }
            other => panic!("unexpected PDU: {other:?}"),
        }
    }
}
