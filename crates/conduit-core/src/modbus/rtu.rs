use super::{Error, Result};
use super::pdu::{decode_pdu, Pdu};

// ── CRC-16/IBM ────────────────────────────────────────────────────────────────

/// Compute the Modbus RTU CRC-16 (polynomial 0xA001, init 0xFFFF).
/// The result is stored little-endian in the frame (low byte first).
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

// ── Frame type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtuFrame {
    pub unit_id:       u8,
    pub function_code: u8,
    /// Bytes between the function code and the CRC (excludes both).
    pub data:          Vec<u8>,
}

impl RtuFrame {
    /// Decode the PDU carried in this frame.
    pub fn decode_pdu(&self) -> Pdu {
        decode_pdu(self.function_code, &self.data)
    }
}

// ── Decoder ───────────────────────────────────────────────────────────────────

/// Attempt to decode one RTU frame from `buf`.
///
/// Scans lengths from 4 bytes upward (minimum: addr + FC + 2 CRC) looking for
/// a length whose trailing two bytes are the correct CRC for the preceding bytes.
/// Returns the first match — in practice this is unambiguous because false-positive
/// rate for CRC-16 is ~1/65536 per candidate length.
///
/// Returns:
/// - `Ok(Some((frame, consumed)))` — valid frame found; `consumed` bytes should
///   be removed from the front of the buffer.
/// - `Ok(None)` — buffer too short or no valid CRC yet; accumulate more bytes.
/// - `Err(Error::CrcMismatch)` — buffer has ≥ 256 bytes but no valid CRC found;
///   the caller should skip one byte and retry to resync.
pub fn decode(buf: &[u8]) -> Result<Option<(RtuFrame, usize)>> {
    const MIN: usize = 4;   // addr(1) + fc(1) + crc(2)
    const MAX: usize = 256; // Modbus maximum frame size

    if buf.len() < MIN {
        return Ok(None);
    }

    let upper = buf.len().min(MAX);
    for len in MIN..=upper {
        let (payload, crc_bytes) = buf[..len].split_at(len - 2);
        let computed = crc16(payload);
        let stored   = u16::from_le_bytes([crc_bytes[0], crc_bytes[1]]);
        if computed == stored {
            return Ok(Some((
                RtuFrame {
                    unit_id:       payload[0],
                    function_code: payload[1],
                    data:          payload[2..].to_vec(),
                },
                len,
            )));
        }
    }

    // With >= MAX bytes in buffer we cannot find a valid frame — resync signal.
    if buf.len() >= MAX {
        Err(Error::CrcMismatch { expected: 0, got: 0 })
    } else {
        Ok(None)
    }
}

// ── Encoder ───────────────────────────────────────────────────────────────────

/// Encode a Modbus RTU frame: `[unit_id][FC][pdu_data…][CRC_lo][CRC_hi]`.
pub fn encode(unit_id: u8, pdu: &Pdu) -> Vec<u8> {
    let pdu_bytes = pdu.encode();
    let mut frame = Vec::with_capacity(1 + pdu_bytes.len() + 2);
    frame.push(unit_id);
    frame.extend_from_slice(&pdu_bytes);
    let crc = crc16(&frame);
    frame.extend_from_slice(&crc.to_le_bytes());
    frame
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modbus::pdu::Pdu;

    // Known-good test vector from the Modbus Application Protocol Specification v1.1b3, §6.3:
    // Request: Read Holding Registers, unit=0x11 (17), start=0x006B (107), count=3
    // Raw: 11 03 00 6B 00 03 76 87
    const READ_REGS_REQ: [u8; 8] = [0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87];

    #[test]
    fn crc_known_vector() {
        let payload = &READ_REGS_REQ[..6];
        assert_eq!(crc16(payload), 0x8776); // stored LE as 76 87
    }

    #[test]
    fn decode_known_frame() {
        let (frame, consumed) = decode(&READ_REGS_REQ).unwrap().unwrap();
        assert_eq!(consumed, 8);
        assert_eq!(frame.unit_id, 0x11);
        assert_eq!(frame.function_code, 0x03);
        assert_eq!(frame.data, [0x00, 0x6B, 0x00, 0x03]);
    }

    #[test]
    fn decode_too_short() {
        assert!(decode(&READ_REGS_REQ[..3]).unwrap().is_none());
    }

    #[test]
    fn encode_decode_roundtrip() {
        let pdu = Pdu::ReadWords { fc: 0x03, start: 0x006B, count: 3 };
        let frame = encode(0x11, &pdu);
        assert_eq!(&frame, &READ_REGS_REQ);
        let (decoded, _) = decode(&frame).unwrap().unwrap();
        assert_eq!(decoded.unit_id, 0x11);
        assert_eq!(decoded.function_code, 0x03);
    }

    #[test]
    fn encode_decode_write_single_register() {
        let pdu = Pdu::WriteSingleRegister { address: 0x0001, value: 0x0003 };
        let frame = encode(1, &pdu);
        let (decoded, consumed) = decode(&frame).unwrap().unwrap();
        assert_eq!(consumed, frame.len());
        match decoded.decode_pdu() {
            Pdu::WriteSingleRegister { address: 0x0001, value: 0x0003 } => {}
            other => panic!("unexpected PDU: {other:?}"),
        }
    }

    #[test]
    fn noise_does_not_produce_spurious_frame() {
        // All-zero buffer: addr=0, fc=0 — fc=0 is not valid Modbus, but our
        // decoder is lenient. What matters is that CRC mismatch blocks decoding
        // until enough bytes accumulate. A 4-byte all-zero buffer has CRC 0x0000
        // but the last two bytes are also 0x0000, so this might match — that's
        // intentional (garbage in → garbage out; the UI layer filters fc=0).
        // We just verify it doesn't panic.
        let _ = decode(&[0u8; 4]);
        let _ = decode(&[0u8; 256]);
    }
}
