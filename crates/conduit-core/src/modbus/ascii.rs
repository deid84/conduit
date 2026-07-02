use super::{Error, Result};
use super::pdu::{decode_pdu, Pdu};

// ── LRC ───────────────────────────────────────────────────────────────────────

/// Compute the Modbus ASCII LRC: two's-complement of the sum of all bytes.
pub fn lrc(data: &[u8]) -> u8 {
    let sum: u8 = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    (!sum).wrapping_add(1)
}

// ── Hex helpers ───────────────────────────────────────────────────────────────

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _           => None,
    }
}

fn from_hex(hi: u8, lo: u8) -> Option<u8> {
    Some((hex_nibble(hi)? << 4) | hex_nibble(lo)?)
}

fn to_hex(byte: u8) -> [u8; 2] {
    const TABLE: &[u8] = b"0123456789ABCDEF";
    [TABLE[(byte >> 4) as usize], TABLE[(byte & 0x0F) as usize]]
}

// ── Frame type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsciiFrame {
    pub unit_id:       u8,
    pub function_code: u8,
    pub data:          Vec<u8>,
}

impl AsciiFrame {
    pub fn decode_pdu(&self) -> Pdu {
        decode_pdu(self.function_code, &self.data)
    }
}

// ── Decoder ───────────────────────────────────────────────────────────────────

/// Attempt to decode one Modbus ASCII frame from `buf`.
///
/// ASCII frame format:
/// ```text
/// ':' [2*N uppercase hex chars] [2 LRC hex chars] CR LF
/// ```
/// Everything between `':'` and CR LF is the hex-encoded payload; the last
/// hex pair is the LRC of the decoded payload bytes preceding it.
///
/// Returns:
/// - `Ok(Some((frame, consumed)))` — valid frame starting from the first `':'`
///   found in `buf`; `consumed` bytes should be removed from the buffer front.
/// - `Ok(None)` — no complete frame yet; accumulate more bytes.
/// - `Err(…)` — malformed frame (bad hex, wrong LRC, too short).
pub fn decode(buf: &[u8]) -> Result<Option<(AsciiFrame, usize)>> {
    // Locate frame start
    let start = match buf.iter().position(|&b| b == b':') {
        Some(p) => p,
        None    => return Ok(None),
    };

    let after_colon = &buf[start + 1..];

    // Locate CRLF
    let crlf_pos = match after_colon.windows(2).position(|w| w == b"\r\n") {
        Some(p) => p,
        None    => return Ok(None),
    };

    let hex_body = &after_colon[..crlf_pos];

    // Minimum: addr(2) + fc(2) + lrc(2) = 6 hex chars
    if hex_body.len() < 6 || hex_body.len() % 2 != 0 {
        return Err(Error::InvalidAscii("hex body too short or odd length"));
    }

    // Decode hex → bytes
    let mut bytes = Vec::with_capacity(hex_body.len() / 2);
    let mut i = 0;
    while i + 1 < hex_body.len() {
        match from_hex(hex_body[i], hex_body[i + 1]) {
            Some(b) => bytes.push(b),
            None    => return Err(Error::InvalidAscii("invalid hex digit")),
        }
        i += 2;
    }

    // Split payload | LRC
    let (payload, lrc_byte) = bytes.split_at(bytes.len() - 1);
    let stored_lrc   = lrc_byte[0];
    let computed_lrc = lrc(payload);
    if computed_lrc != stored_lrc {
        return Err(Error::LrcMismatch { expected: stored_lrc, got: computed_lrc });
    }

    if payload.len() < 2 {
        return Err(Error::InvalidAscii("payload too short after LRC check"));
    }

    let consumed = start + 1 + crlf_pos + 2; // ':' + hex + CR LF
    Ok(Some((
        AsciiFrame {
            unit_id:       payload[0],
            function_code: payload[1],
            data:          payload[2..].to_vec(),
        },
        consumed,
    )))
}

// ── Encoder ───────────────────────────────────────────────────────────────────

/// Encode a Modbus ASCII frame.
///
/// Output: `':'` + uppercase hex pairs + LRC hex pair + `"\r\n"`.
pub fn encode(unit_id: u8, pdu: &Pdu) -> Vec<u8> {
    let pdu_bytes = pdu.encode();

    // Build the payload (unit_id + PDU bytes) and compute LRC
    let mut payload = Vec::with_capacity(1 + pdu_bytes.len());
    payload.push(unit_id);
    payload.extend_from_slice(&pdu_bytes);
    let lrc_byte = lrc(&payload);

    // Encode as ASCII: ':' + hex(payload) + hex(LRC) + CRLF
    let mut frame = vec![b':'];
    for &b in &payload {
        frame.extend_from_slice(&to_hex(b));
    }
    frame.extend_from_slice(&to_hex(lrc_byte));
    frame.extend_from_slice(b"\r\n");
    frame
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modbus::pdu::Pdu;

    // Known-good vector from the Modbus specification:
    // Request: Read Coils, unit=0x0B, start=0x0013 (19), count=0x0025 (37)
    // LRC of [0x0B, 0x01, 0x00, 0x13, 0x00, 0x25]:
    //   sum = 0x44, LRC = !0x44 + 1 = 0xBC
    // Frame: :0B0100130025BC\r\n
    const READ_COILS_REQ: &[u8] = b":0B0100130025BC\r\n";

    #[test]
    fn lrc_known_vector() {
        let payload = [0x0Bu8, 0x01, 0x00, 0x13, 0x00, 0x25];
        assert_eq!(lrc(&payload), 0xBC);
    }

    #[test]
    fn decode_known_frame() {
        let (frame, consumed) = decode(READ_COILS_REQ).unwrap().unwrap();
        assert_eq!(consumed, READ_COILS_REQ.len());
        assert_eq!(frame.unit_id, 0x0B);
        assert_eq!(frame.function_code, 0x01);
        assert_eq!(frame.data, [0x00, 0x13, 0x00, 0x25]);
    }

    #[test]
    fn decode_incomplete_returns_none() {
        assert!(decode(b":0B010013").unwrap().is_none());
    }

    #[test]
    fn decode_bad_lrc_returns_err() {
        // Corrupt the LRC byte (BC → BD)
        let bad: Vec<u8> = b":0B0100130025BD\r\n".to_vec();
        assert!(matches!(decode(&bad), Err(Error::LrcMismatch { .. })));
    }

    #[test]
    fn encode_decode_roundtrip() {
        let pdu = Pdu::ReadBits { fc: 0x01, start: 0x0013, count: 0x0025 };
        let raw = encode(0x0B, &pdu);
        assert_eq!(raw, READ_COILS_REQ);
        let (frame, _) = decode(&raw).unwrap().unwrap();
        assert_eq!(frame.unit_id, 0x0B);
        match frame.decode_pdu() {
            Pdu::ReadBits { fc: 0x01, start: 0x0013, count: 0x0025 } => {}
            other => panic!("unexpected PDU: {other:?}"),
        }
    }

    #[test]
    fn encode_decode_write_single_register() {
        let pdu = Pdu::WriteSingleRegister { address: 0x0001, value: 0x03E8 };
        let raw = encode(1, &pdu);
        let (frame, _) = decode(&raw).unwrap().unwrap();
        assert_eq!(frame.unit_id, 1);
        match frame.decode_pdu() {
            Pdu::WriteSingleRegister { address: 0x0001, value: 0x03E8 } => {}
            other => panic!("unexpected PDU: {other:?}"),
        }
    }

    #[test]
    fn decode_skips_garbage_before_colon() {
        let mut buf = b"garbage data\xff:0B0100130025BC\r\n".to_vec();
        let (frame, _) = decode(&mut buf).unwrap().unwrap();
        assert_eq!(frame.unit_id, 0x0B);
    }
}
