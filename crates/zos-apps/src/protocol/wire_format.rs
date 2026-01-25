//! Wire Format
//!
//! Binary encoding/decoding for protocol messages.

use crate::framework::ProtocolError;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// Protocol version byte
pub const PROTOCOL_VERSION: u8 = 0x01;

/// Message envelope (header + payload)
#[derive(Clone, Debug)]
pub struct Envelope {
    /// Protocol version
    pub version: u8,
    /// Type tag identifying the payload type
    pub type_tag: u8,
    /// Payload data
    pub payload: Vec<u8>,
}

impl Envelope {
    /// Create a new envelope
    pub fn new(type_tag: u8, payload: Vec<u8>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            type_tag,
            payload,
        }
    }
}

/// Encode an envelope to bytes
pub fn encode_envelope(envelope: &Envelope) -> Vec<u8> {
    let payload_len = envelope.payload.len() as u16;
    let mut bytes = Vec::with_capacity(4 + envelope.payload.len());

    bytes.push(envelope.version);
    bytes.push(envelope.type_tag);
    bytes.extend_from_slice(&payload_len.to_le_bytes());
    bytes.extend_from_slice(&envelope.payload);

    bytes
}

/// Decode an envelope from bytes
pub fn decode_envelope(data: &[u8]) -> Result<Envelope, ProtocolError> {
    // Check minimum header size
    if data.len() < 4 {
        return Err(ProtocolError::TooShort);
    }

    let version = data[0];
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnknownVersion(version));
    }

    let type_tag = data[1];
    let payload_len = u16::from_le_bytes([data[2], data[3]]) as usize;

    if data.len() < 4 + payload_len {
        return Err(ProtocolError::PayloadOverflow {
            declared: payload_len,
            available: data.len() - 4,
        });
    }

    let payload = data[4..4 + payload_len].to_vec();

    Ok(Envelope {
        version,
        type_tag,
        payload,
    })
}

// ============================================================================
// String Encoding Helpers
// ============================================================================

/// Encode a string as length-prefixed UTF-8 (u16 length)
pub fn encode_string(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let len = bytes.len() as u16;
    let mut result = Vec::with_capacity(2 + bytes.len());
    result.extend_from_slice(&len.to_le_bytes());
    result.extend_from_slice(bytes);
    result
}

/// Decode a length-prefixed string from data at the given cursor position
pub fn decode_string(data: &[u8], cursor: &mut usize) -> Result<String, ProtocolError> {
    if *cursor + 2 > data.len() {
        return Err(ProtocolError::TooShort);
    }

    let len = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]) as usize;
    *cursor += 2;

    if *cursor + len > data.len() {
        return Err(ProtocolError::StringOverflow {
            declared: len,
            available: data.len() - *cursor,
        });
    }

    let bytes = &data[*cursor..*cursor + len];
    *cursor += len;

    String::from_utf8(bytes.to_vec()).map_err(|_| ProtocolError::InvalidUtf8)
}

/// Decode a u8 from data at the given cursor position
pub fn decode_u8(data: &[u8], cursor: &mut usize) -> Result<u8, ProtocolError> {
    if *cursor >= data.len() {
        return Err(ProtocolError::TooShort);
    }
    let value = data[*cursor];
    *cursor += 1;
    Ok(value)
}

/// Decode a u16 (little-endian) from data at the given cursor position
pub fn decode_u16(data: &[u8], cursor: &mut usize) -> Result<u16, ProtocolError> {
    if *cursor + 2 > data.len() {
        return Err(ProtocolError::TooShort);
    }
    let value = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
    *cursor += 2;
    Ok(value)
}

/// Decode a u32 (little-endian) from data at the given cursor position
pub fn decode_u32(data: &[u8], cursor: &mut usize) -> Result<u32, ProtocolError> {
    if *cursor + 4 > data.len() {
        return Err(ProtocolError::TooShort);
    }
    let value = u32::from_le_bytes([
        data[*cursor],
        data[*cursor + 1],
        data[*cursor + 2],
        data[*cursor + 3],
    ]);
    *cursor += 4;
    Ok(value)
}

/// Decode an optional char (0x00 = None, 0x01 + u32 = Some(char))
pub fn decode_optional_char(
    data: &[u8],
    cursor: &mut usize,
) -> Result<Option<char>, ProtocolError> {
    let has_value = decode_u8(data, cursor)?;
    if has_value == 0 {
        Ok(None)
    } else {
        let code = decode_u32(data, cursor)?;
        Ok(char::from_u32(code))
    }
}

/// Encode an optional char
pub fn encode_optional_char(c: Option<char>) -> Vec<u8> {
    match c {
        None => vec![0x00],
        Some(ch) => {
            let mut bytes = vec![0x01];
            bytes.extend_from_slice(&(ch as u32).to_le_bytes());
            bytes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_roundtrip() {
        let envelope = Envelope::new(0x01, vec![1, 2, 3, 4, 5]);
        let encoded = encode_envelope(&envelope);
        let decoded = decode_envelope(&encoded).unwrap();

        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.type_tag, 0x01);
        assert_eq!(decoded.payload, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_string_roundtrip() {
        let s = "Hello, World!";
        let encoded = encode_string(s);
        let mut cursor = 0;
        let decoded = decode_string(&encoded, &mut cursor).unwrap();

        assert_eq!(decoded, s);
        assert_eq!(cursor, encoded.len());
    }

    #[test]
    fn test_optional_char_roundtrip() {
        // None case
        let encoded = encode_optional_char(None);
        let mut cursor = 0;
        let decoded = decode_optional_char(&encoded, &mut cursor).unwrap();
        assert_eq!(decoded, None);

        // Some case
        let encoded = encode_optional_char(Some('+'));
        let mut cursor = 0;
        let decoded = decode_optional_char(&encoded, &mut cursor).unwrap();
        assert_eq!(decoded, Some('+'));
    }

    // ========================================================================
    // Edge case tests for protocol parsing
    // ========================================================================

    #[test]
    fn test_envelope_too_short_header() {
        // Less than 4 bytes (minimum header size)
        let result = decode_envelope(&[0x01, 0x02]);
        assert!(matches!(result, Err(ProtocolError::TooShort)));

        let result = decode_envelope(&[]);
        assert!(matches!(result, Err(ProtocolError::TooShort)));
    }

    #[test]
    fn test_envelope_invalid_version() {
        // Wrong version byte
        let data = [0x00, 0x01, 0x00, 0x00]; // version 0 instead of 1
        let result = decode_envelope(&data);
        assert!(matches!(result, Err(ProtocolError::UnknownVersion(0))));

        let data = [0xFF, 0x01, 0x00, 0x00]; // version 255
        let result = decode_envelope(&data);
        assert!(matches!(result, Err(ProtocolError::UnknownVersion(0xFF))));
    }

    #[test]
    fn test_envelope_truncated_payload() {
        // Header claims 10 bytes payload, but only 3 available
        let data = [PROTOCOL_VERSION, 0x01, 0x0A, 0x00, 0x01, 0x02, 0x03];
        let result = decode_envelope(&data);
        assert!(matches!(
            result,
            Err(ProtocolError::PayloadOverflow {
                declared: 10,
                available: 3
            })
        ));
    }

    #[test]
    fn test_envelope_empty_payload() {
        // Valid envelope with zero-length payload
        let data = [PROTOCOL_VERSION, 0x01, 0x00, 0x00];
        let result = decode_envelope(&data).unwrap();
        assert!(result.payload.is_empty());
    }

    #[test]
    fn test_envelope_max_payload() {
        // Maximum 16-bit payload length
        let payload = vec![0xAB; 65535];
        let envelope = Envelope::new(0x42, payload.clone());
        let encoded = encode_envelope(&envelope);
        let decoded = decode_envelope(&encoded).unwrap();
        assert_eq!(decoded.payload.len(), 65535);
    }

    #[test]
    fn test_string_truncated_length() {
        // Only 1 byte when 2 needed for length prefix
        let data = [0x05];
        let mut cursor = 0;
        let result = decode_string(&data, &mut cursor);
        assert!(matches!(result, Err(ProtocolError::TooShort)));
    }

    #[test]
    fn test_string_truncated_data() {
        // Length says 10 bytes, but only 3 available
        let data = [0x0A, 0x00, b'a', b'b', b'c'];
        let mut cursor = 0;
        let result = decode_string(&data, &mut cursor);
        assert!(matches!(
            result,
            Err(ProtocolError::StringOverflow {
                declared: 10,
                available: 3
            })
        ));
    }

    #[test]
    fn test_string_invalid_utf8() {
        // Invalid UTF-8 sequence
        let data = [0x04, 0x00, 0xFF, 0xFE, 0xFD, 0xFC];
        let mut cursor = 0;
        let result = decode_string(&data, &mut cursor);
        assert!(matches!(result, Err(ProtocolError::InvalidUtf8)));
    }

    #[test]
    fn test_string_empty() {
        // Valid empty string
        let data = [0x00, 0x00];
        let mut cursor = 0;
        let result = decode_string(&data, &mut cursor).unwrap();
        assert_eq!(result, "");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn test_string_unicode() {
        // Unicode string with multi-byte characters
        let s = "Hello, 世界! 🌍";
        let encoded = encode_string(s);
        let mut cursor = 0;
        let decoded = decode_string(&encoded, &mut cursor).unwrap();
        assert_eq!(decoded, s);
    }

    #[test]
    fn test_u8_at_end_of_buffer() {
        let data = [];
        let mut cursor = 0;
        let result = decode_u8(&data, &mut cursor);
        assert!(matches!(result, Err(ProtocolError::TooShort)));
    }

    #[test]
    fn test_u16_truncated() {
        let data = [0x42];
        let mut cursor = 0;
        let result = decode_u16(&data, &mut cursor);
        assert!(matches!(result, Err(ProtocolError::TooShort)));
    }

    #[test]
    fn test_u32_truncated() {
        let data = [0x01, 0x02];
        let mut cursor = 0;
        let result = decode_u32(&data, &mut cursor);
        assert!(matches!(result, Err(ProtocolError::TooShort)));
    }

    #[test]
    fn test_optional_char_truncated() {
        // Has value flag, but missing char data
        let data = [0x01, 0x00];
        let mut cursor = 0;
        let result = decode_optional_char(&data, &mut cursor);
        assert!(matches!(result, Err(ProtocolError::TooShort)));
    }

    #[test]
    fn test_optional_char_invalid_codepoint() {
        // Invalid Unicode code point (surrogate)
        let data = [0x01, 0x00, 0xD8, 0x00, 0x00]; // U+D800 (surrogate)
        let mut cursor = 0;
        let result = decode_optional_char(&data, &mut cursor).unwrap();
        // char::from_u32 returns None for invalid code points
        assert_eq!(result, None);
    }

    #[test]
    fn test_multiple_strings_sequential() {
        // Multiple strings in sequence
        let s1 = "first";
        let s2 = "second";
        let mut data = encode_string(s1);
        data.extend_from_slice(&encode_string(s2));

        let mut cursor = 0;
        let decoded1 = decode_string(&data, &mut cursor).unwrap();
        let decoded2 = decode_string(&data, &mut cursor).unwrap();

        assert_eq!(decoded1, s1);
        assert_eq!(decoded2, s2);
        assert_eq!(cursor, data.len());
    }
}
