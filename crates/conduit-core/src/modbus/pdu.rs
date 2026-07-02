use serde::{Deserialize, Serialize};

// ── Function codes ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionCode {
    ReadCoils              = 0x01,
    ReadDiscreteInputs     = 0x02,
    ReadHoldingRegisters   = 0x03,
    ReadInputRegisters     = 0x04,
    WriteSingleCoil        = 0x05,
    WriteSingleRegister    = 0x06,
    WriteMultipleCoils     = 0x0F,
    WriteMultipleRegisters = 0x10,
    ReadWriteMultipleRegs  = 0x17,
}

impl FunctionCode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::ReadCoils),
            0x02 => Some(Self::ReadDiscreteInputs),
            0x03 => Some(Self::ReadHoldingRegisters),
            0x04 => Some(Self::ReadInputRegisters),
            0x05 => Some(Self::WriteSingleCoil),
            0x06 => Some(Self::WriteSingleRegister),
            0x0F => Some(Self::WriteMultipleCoils),
            0x10 => Some(Self::WriteMultipleRegisters),
            0x17 => Some(Self::ReadWriteMultipleRegs),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 { self as u8 }
}

// ── Exception codes ───────────────────────────────────────────────────────────

/// Newtype over the raw exception code byte so that unknown values are preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExceptionCode(pub u8);

impl ExceptionCode {
    pub const ILLEGAL_FUNCTION:         Self = Self(0x01);
    pub const ILLEGAL_DATA_ADDRESS:     Self = Self(0x02);
    pub const ILLEGAL_DATA_VALUE:       Self = Self(0x03);
    pub const SERVER_DEVICE_FAILURE:    Self = Self(0x04);
    pub const ACKNOWLEDGE:              Self = Self(0x05);
    pub const SERVER_DEVICE_BUSY:       Self = Self(0x06);
    pub const MEMORY_PARITY_ERROR:      Self = Self(0x08);
    pub const GATEWAY_PATH_UNAVAILABLE: Self = Self(0x0A);
    pub const GATEWAY_NO_RESPONSE:      Self = Self(0x0B);

    pub fn description(self) -> &'static str {
        match self.0 {
            0x01 => "Illegal Function",
            0x02 => "Illegal Data Address",
            0x03 => "Illegal Data Value",
            0x04 => "Server Device Failure",
            0x05 => "Acknowledge",
            0x06 => "Server Device Busy",
            0x08 => "Memory Parity Error",
            0x0A => "Gateway Path Unavailable",
            0x0B => "Gateway Target Device Failed to Respond",
            _    => "Unknown Exception",
        }
    }
}

impl std::fmt::Display for ExceptionCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({:#04x})", self.description(), self.0)
    }
}

// ── PDU ───────────────────────────────────────────────────────────────────────

/// Modbus Protocol Data Unit — function code + data, without transport framing.
///
/// For FC 01-04 the same wire format encodes both requests and responses;
/// `decode_pdu` uses data-length heuristics to select the correct variant.
/// FC 05/06 request and response are byte-identical so one variant covers both.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Pdu {
    // ── Read requests (FC 01-04) ──────────────────────────────────────────
    ReadBits  { fc: u8, start: u16, count: u16 },
    ReadWords { fc: u8, start: u16, count: u16 },

    // ── Read responses ────────────────────────────────────────────────────
    ReadBitsResponse  { fc: u8, values: Vec<bool> },
    ReadWordsResponse { fc: u8, values: Vec<u16>  },

    // ── Single write — req and resp are wire-identical ────────────────────
    WriteSingleCoil     { address: u16, on:    bool },
    WriteSingleRegister { address: u16, value: u16  },

    // ── Multiple-write request ────────────────────────────────────────────
    WriteMultipleCoils     { address: u16, count: u16, values: Vec<bool> },
    WriteMultipleRegisters { address: u16, count: u16, values: Vec<u16>  },

    // ── Multiple-write response (FC 0F / 10) ──────────────────────────────
    WriteMultipleResponse { fc: u8, address: u16, count: u16 },

    // ── Read/Write multiple registers (FC 17) ─────────────────────────────
    ReadWriteMultiple {
        read_address:  u16,
        read_count:    u16,
        write_address: u16,
        write_values:  Vec<u16>,
    },
    ReadWriteMultipleResponse { values: Vec<u16> },

    // ── Exception ─────────────────────────────────────────────────────────
    Exception { function_code: u8, exception_code: ExceptionCode },

    // ── Unknown / unsupported FC ──────────────────────────────────────────
    Unknown { function_code: u8, data: Vec<u8> },
}

impl Pdu {
    /// Raw function-code byte for this PDU (without the exception bit).
    pub fn function_code_u8(&self) -> u8 {
        match self {
            Pdu::ReadBits  { fc, .. } | Pdu::ReadBitsResponse  { fc, .. } |
            Pdu::ReadWords { fc, .. } | Pdu::ReadWordsResponse { fc, .. } |
            Pdu::WriteMultipleResponse { fc, .. } => *fc,
            Pdu::WriteSingleCoil { .. }                => 0x05,
            Pdu::WriteSingleRegister { .. }            => 0x06,
            Pdu::WriteMultipleCoils { .. }             => 0x0F,
            Pdu::WriteMultipleRegisters { .. }         => 0x10,
            Pdu::ReadWriteMultiple { .. } |
            Pdu::ReadWriteMultipleResponse { .. }      => 0x17,
            Pdu::Exception { function_code, .. }       => *function_code,
            Pdu::Unknown   { function_code, .. }       => *function_code,
        }
    }

    /// Human-readable name for this PDU's function code.
    pub fn function_name(&self) -> &'static str {
        match self.function_code_u8() {
            0x01 => "Read Coils",
            0x02 => "Read Discrete Inputs",
            0x03 => "Read Holding Registers",
            0x04 => "Read Input Registers",
            0x05 => "Write Single Coil",
            0x06 => "Write Single Register",
            0x0F => "Write Multiple Coils",
            0x10 => "Write Multiple Registers",
            0x17 => "Read/Write Multiple Registers",
            _    => "Unknown",
        }
    }

    /// Encode this PDU to wire bytes: `[FC][data…]`.
    ///
    /// Response-only variants encode a best-effort representation; in practice
    /// only request variants are built programmatically.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Pdu::ReadBits  { fc, start, count } |
            Pdu::ReadWords { fc, start, count } => {
                let mut v = vec![*fc];
                v.extend_from_slice(&start.to_be_bytes());
                v.extend_from_slice(&count.to_be_bytes());
                v
            }

            Pdu::WriteSingleCoil { address, on } => {
                let mut v = vec![0x05];
                v.extend_from_slice(&address.to_be_bytes());
                v.extend_from_slice(if *on { &[0xFF, 0x00] } else { &[0x00, 0x00] });
                v
            }

            Pdu::WriteSingleRegister { address, value } => {
                let mut v = vec![0x06];
                v.extend_from_slice(&address.to_be_bytes());
                v.extend_from_slice(&value.to_be_bytes());
                v
            }

            Pdu::WriteMultipleCoils { address, count, values } => {
                let byte_count = (values.len() + 7) / 8;
                let mut v = vec![0x0F];
                v.extend_from_slice(&address.to_be_bytes());
                v.extend_from_slice(&count.to_be_bytes());
                v.push(byte_count as u8);
                v.extend(pack_bits(values));
                v
            }

            Pdu::WriteMultipleRegisters { address, count, values } => {
                let byte_count = values.len() * 2;
                let mut v = vec![0x10];
                v.extend_from_slice(&address.to_be_bytes());
                v.extend_from_slice(&count.to_be_bytes());
                v.push(byte_count as u8);
                for &reg in values {
                    v.extend_from_slice(&reg.to_be_bytes());
                }
                v
            }

            Pdu::ReadWriteMultiple { read_address, read_count, write_address, write_values } => {
                let write_count = write_values.len() as u16;
                let byte_count  = write_values.len() * 2;
                let mut v = vec![0x17];
                v.extend_from_slice(&read_address.to_be_bytes());
                v.extend_from_slice(&read_count.to_be_bytes());
                v.extend_from_slice(&write_address.to_be_bytes());
                v.extend_from_slice(&write_count.to_be_bytes());
                v.push(byte_count as u8);
                for &reg in write_values {
                    v.extend_from_slice(&reg.to_be_bytes());
                }
                v
            }

            Pdu::WriteMultipleResponse { fc, address, count } => {
                let mut v = vec![*fc];
                v.extend_from_slice(&address.to_be_bytes());
                v.extend_from_slice(&count.to_be_bytes());
                v
            }

            Pdu::ReadWriteMultipleResponse { values } => {
                let byte_count = values.len() * 2;
                let mut v = vec![0x17, byte_count as u8];
                for &reg in values {
                    v.extend_from_slice(&reg.to_be_bytes());
                }
                v
            }

            Pdu::Exception { function_code, exception_code } => {
                vec![function_code | 0x80, exception_code.0]
            }

            // Response-only or unknown: encode raw
            Pdu::ReadBitsResponse  { fc, .. } |
            Pdu::ReadWordsResponse { fc, .. }  => vec![*fc],

            Pdu::Unknown { function_code, data } => {
                let mut v = vec![*function_code];
                v.extend_from_slice(data);
                v
            }
        }
    }
}

// ── Public decode entry point ─────────────────────────────────────────────────

/// Decode a PDU from raw bytes following the function code in a Modbus frame.
///
/// `fc_byte`  — raw function-code byte (may have bit 7 set for exceptions).
/// `data`     — bytes after the FC within the frame (excludes FC itself).
pub fn decode_pdu(fc_byte: u8, data: &[u8]) -> Pdu {
    if fc_byte & 0x80 != 0 {
        let base_fc = fc_byte & 0x7F;
        let exc = data.first().copied().map(ExceptionCode).unwrap_or(ExceptionCode(0));
        return Pdu::Exception { function_code: base_fc, exception_code: exc };
    }

    match fc_byte {
        fc @ (0x01 | 0x02) => decode_read_bits(fc, data),
        fc @ (0x03 | 0x04) => decode_read_words(fc, data),
        0x05               => decode_write_single_coil(data),
        0x06               => decode_write_single_register(data),
        0x0F               => decode_write_multiple_coils(data),
        0x10               => decode_write_multiple_registers(data),
        0x17               => decode_read_write_multiple(data),
        _                  => Pdu::Unknown { function_code: fc_byte, data: data.to_vec() },
    }
}

// ── Private decoders ──────────────────────────────────────────────────────────

fn decode_read_bits(fc: u8, data: &[u8]) -> Pdu {
    match data.len() {
        // 4 bytes = request: start_addr(2) + quantity(2)
        4 => Pdu::ReadBits {
            fc,
            start: u16::from_be_bytes([data[0], data[1]]),
            count: u16::from_be_bytes([data[2], data[3]]),
        },
        // 1 + byte_count bytes = response
        n if n >= 1 => {
            let byte_count = data[0] as usize;
            if n == 1 + byte_count {
                Pdu::ReadBitsResponse { fc, values: unpack_bits(&data[1..]) }
            } else {
                Pdu::Unknown { function_code: fc, data: data.to_vec() }
            }
        }
        _ => Pdu::Unknown { function_code: fc, data: data.to_vec() },
    }
}

fn decode_read_words(fc: u8, data: &[u8]) -> Pdu {
    match data.len() {
        4 => Pdu::ReadWords {
            fc,
            start: u16::from_be_bytes([data[0], data[1]]),
            count: u16::from_be_bytes([data[2], data[3]]),
        },
        n if n >= 1 => {
            let byte_count = data[0] as usize;
            if n == 1 + byte_count && byte_count % 2 == 0 {
                let values = data[1..].chunks(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                Pdu::ReadWordsResponse { fc, values }
            } else {
                Pdu::Unknown { function_code: fc, data: data.to_vec() }
            }
        }
        _ => Pdu::Unknown { function_code: fc, data: data.to_vec() },
    }
}

fn decode_write_single_coil(data: &[u8]) -> Pdu {
    if data.len() == 4 {
        Pdu::WriteSingleCoil {
            address: u16::from_be_bytes([data[0], data[1]]),
            on:      u16::from_be_bytes([data[2], data[3]]) == 0xFF00,
        }
    } else {
        Pdu::Unknown { function_code: 0x05, data: data.to_vec() }
    }
}

fn decode_write_single_register(data: &[u8]) -> Pdu {
    if data.len() == 4 {
        Pdu::WriteSingleRegister {
            address: u16::from_be_bytes([data[0], data[1]]),
            value:   u16::from_be_bytes([data[2], data[3]]),
        }
    } else {
        Pdu::Unknown { function_code: 0x06, data: data.to_vec() }
    }
}

fn decode_write_multiple_coils(data: &[u8]) -> Pdu {
    match data.len() {
        4 => Pdu::WriteMultipleResponse {
            fc:      0x0F,
            address: u16::from_be_bytes([data[0], data[1]]),
            count:   u16::from_be_bytes([data[2], data[3]]),
        },
        n if n >= 5 => {
            let address    = u16::from_be_bytes([data[0], data[1]]);
            let count      = u16::from_be_bytes([data[2], data[3]]);
            let byte_count = data[4] as usize;
            if n == 5 + byte_count {
                Pdu::WriteMultipleCoils {
                    address,
                    count,
                    values: unpack_bits_n(&data[5..], count as usize),
                }
            } else {
                Pdu::Unknown { function_code: 0x0F, data: data.to_vec() }
            }
        }
        _ => Pdu::Unknown { function_code: 0x0F, data: data.to_vec() },
    }
}

fn decode_write_multiple_registers(data: &[u8]) -> Pdu {
    match data.len() {
        4 => Pdu::WriteMultipleResponse {
            fc:      0x10,
            address: u16::from_be_bytes([data[0], data[1]]),
            count:   u16::from_be_bytes([data[2], data[3]]),
        },
        n if n >= 5 => {
            let address    = u16::from_be_bytes([data[0], data[1]]);
            let count      = u16::from_be_bytes([data[2], data[3]]);
            let byte_count = data[4] as usize;
            if n == 5 + byte_count && byte_count % 2 == 0 {
                let values = data[5..].chunks(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                Pdu::WriteMultipleRegisters { address, count, values }
            } else {
                Pdu::Unknown { function_code: 0x10, data: data.to_vec() }
            }
        }
        _ => Pdu::Unknown { function_code: 0x10, data: data.to_vec() },
    }
}

fn decode_read_write_multiple(data: &[u8]) -> Pdu {
    // Response heuristic: first byte = byte_count, rest = byte_count bytes of register data
    if data.len() >= 1 {
        let byte_count = data[0] as usize;
        if data.len() == 1 + byte_count && byte_count % 2 == 0 {
            let values = data[1..].chunks(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            return Pdu::ReadWriteMultipleResponse { values };
        }
    }
    // Request: read_addr(2) + read_count(2) + write_addr(2) + write_count(2) + byte_count(1) + values
    if data.len() >= 9 {
        let read_address  = u16::from_be_bytes([data[0], data[1]]);
        let read_count    = u16::from_be_bytes([data[2], data[3]]);
        let write_address = u16::from_be_bytes([data[4], data[5]]);
        let _write_count  = u16::from_be_bytes([data[6], data[7]]);
        let byte_count    = data[8] as usize;
        if data.len() == 9 + byte_count && byte_count % 2 == 0 {
            let write_values = data[9..].chunks(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            return Pdu::ReadWriteMultiple { read_address, read_count, write_address, write_values };
        }
    }
    Pdu::Unknown { function_code: 0x17, data: data.to_vec() }
}

// ── Bit packing helpers ───────────────────────────────────────────────────────

/// Unpack all bits from bytes, LSB-first per Modbus spec.
pub fn unpack_bits(bytes: &[u8]) -> Vec<bool> {
    bytes.iter().flat_map(|&b| (0..8).map(move |i| (b >> i) & 1 != 0)).collect()
}

/// Unpack exactly `count` bits.
pub fn unpack_bits_n(bytes: &[u8], count: usize) -> Vec<bool> {
    unpack_bits(bytes).into_iter().take(count).collect()
}

/// Pack bits into bytes, LSB-first per Modbus spec.
pub fn pack_bits(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8).map(|chunk| {
        chunk.iter().enumerate().fold(0u8, |acc, (i, &b)| {
            if b { acc | (1 << i) } else { acc }
        })
    }).collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_roundtrip() {
        let bits = vec![true, false, true, true, false, false, true, false, true];
        let packed   = pack_bits(&bits);
        let unpacked = unpack_bits_n(&packed, bits.len());
        assert_eq!(bits, unpacked);
    }

    #[test]
    fn decode_read_holding_registers_request() {
        // FC 03, start=0x006B, count=3
        let data = [0x00u8, 0x6B, 0x00, 0x03];
        match decode_pdu(0x03, &data) {
            Pdu::ReadWords { fc: 0x03, start: 0x006B, count: 3 } => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn decode_read_holding_registers_response() {
        // FC 03, byte_count=6, three registers: 0x022B, 0x0000, 0x0064
        let data = [0x06u8, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
        match decode_pdu(0x03, &data) {
            Pdu::ReadWordsResponse { fc: 0x03, values } => {
                assert_eq!(values, vec![0x022B, 0x0000, 0x0064]);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn decode_write_single_coil() {
        let data = [0x00u8, 0xAC, 0xFF, 0x00];
        match decode_pdu(0x05, &data) {
            Pdu::WriteSingleCoil { address: 0x00AC, on: true } => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn decode_exception() {
        // FC 03 exception, code 0x02
        let data = [0x02u8];
        match decode_pdu(0x83, &data) {
            Pdu::Exception { function_code: 0x03, exception_code } => {
                assert_eq!(exception_code, ExceptionCode::ILLEGAL_DATA_ADDRESS);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn encode_decode_roundtrip_read_words() {
        let pdu = Pdu::ReadWords { fc: 0x03, start: 0x006B, count: 3 };
        let encoded = pdu.encode();
        assert_eq!(encoded, [0x03, 0x00, 0x6B, 0x00, 0x03]);
        let decoded = decode_pdu(encoded[0], &encoded[1..]);
        matches!(decoded, Pdu::ReadWords { .. });
    }

    #[test]
    fn encode_decode_write_single_coil() {
        let pdu = Pdu::WriteSingleCoil { address: 0x00AC, on: true };
        let encoded = pdu.encode();
        assert_eq!(encoded, [0x05, 0x00, 0xAC, 0xFF, 0x00]);
        match decode_pdu(encoded[0], &encoded[1..]) {
            Pdu::WriteSingleCoil { address: 0x00AC, on: true } => {}
            other => panic!("unexpected: {other:?}"),
        }
    }
}
