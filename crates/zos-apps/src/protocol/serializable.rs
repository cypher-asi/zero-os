//! Wire Serializable Trait
//!
//! Provides a standardized interface for types that can be serialized
//! to/from the wire protocol format.

use super::wire_format::{decode_envelope, encode_envelope, Envelope};
use crate::framework::ProtocolError;
use alloc::vec::Vec;

/// Trait for types that can be serialized to/from the wire protocol.
///
/// This trait standardizes the serialization pattern used by all app state types,
/// reducing boilerplate and ensuring consistent envelope handling.
///
/// # Wire Format
///
/// All messages are wrapped in an envelope:
///
/// ```text
/// ┌─────────┬──────────┬─────────────┬─────────────────────┐
/// │ version │ type_tag │ payload_len │       payload       │
/// │  (u8)   │   (u8)   │    (u16)    │      (bytes)        │
/// └─────────┴──────────┴─────────────┴─────────────────────┘
///    1 byte    1 byte     2 bytes      0-65535 bytes
/// ```
///
/// The payload starts with the type tag byte, followed by type-specific data.
///
/// # Example
///
/// ```ignore
/// use zos_apps::protocol::{WireSerializable, type_tags::TYPE_CLOCK_STATE};
///
/// impl WireSerializable for ClockState {
///     const TYPE_TAG: u8 = TYPE_CLOCK_STATE;
///
///     fn encode_payload(&self) -> Vec<u8> {
///         let mut payload = vec![Self::TYPE_TAG];
///         // ... encode fields ...
///         payload
///     }
///
///     fn decode_payload(payload: &[u8]) -> Result<Self, ProtocolError> {
///         let mut cursor = 1; // skip type tag
///         // ... decode fields ...
///     }
/// }
/// ```
pub trait WireSerializable: Sized {
    /// The type tag byte that identifies this message type in the wire format.
    const TYPE_TAG: u8;

    /// Encode the type-specific payload.
    ///
    /// The payload MUST start with `Self::TYPE_TAG` as the first byte.
    fn encode_payload(&self) -> Vec<u8>;

    /// Decode from the payload bytes.
    ///
    /// The `payload` includes the leading type tag byte (already verified to match).
    /// Implementations should start parsing at `cursor = 1`.
    fn decode_payload(payload: &[u8]) -> Result<Self, ProtocolError>;

    /// Serialize to wire format bytes.
    ///
    /// This wraps the payload in an envelope with version and length header.
    fn to_bytes(&self) -> Vec<u8> {
        let payload = self.encode_payload();
        let envelope = Envelope::new(Self::TYPE_TAG, payload);
        encode_envelope(&envelope)
    }

    /// Deserialize from wire format bytes.
    ///
    /// This unwraps the envelope and verifies the type tag before decoding.
    fn from_bytes(data: &[u8]) -> Result<Self, ProtocolError> {
        let envelope = decode_envelope(data)?;

        // Verify type tag matches
        if envelope.type_tag != Self::TYPE_TAG {
            return Err(ProtocolError::UnexpectedType {
                expected: Self::TYPE_TAG,
                got: envelope.type_tag,
            });
        }

        // Check payload is non-empty
        if envelope.payload.is_empty() {
            return Err(ProtocolError::EmptyPayload);
        }

        Self::decode_payload(&envelope.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Test type for verifying trait behavior
    #[derive(Debug, PartialEq)]
    struct TestState {
        value: u8,
    }

    impl WireSerializable for TestState {
        const TYPE_TAG: u8 = 0xAB;

        fn encode_payload(&self) -> Vec<u8> {
            vec![Self::TYPE_TAG, self.value]
        }

        fn decode_payload(payload: &[u8]) -> Result<Self, ProtocolError> {
            if payload.len() < 2 {
                return Err(ProtocolError::TooShort);
            }
            Ok(TestState { value: payload[1] })
        }
    }

    #[test]
    fn test_wire_serializable_roundtrip() {
        let state = TestState { value: 42 };
        let bytes = state.to_bytes();
        let decoded = TestState::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, state);
    }

    #[test]
    fn test_wire_serializable_wrong_type_tag() {
        // Create bytes with wrong type tag
        let envelope = Envelope::new(0xFF, vec![0xFF, 42]);
        let bytes = encode_envelope(&envelope);

        let result = TestState::from_bytes(&bytes);
        assert!(matches!(
            result,
            Err(ProtocolError::UnexpectedType {
                expected: 0xAB,
                got: 0xFF
            })
        ));
    }

    #[test]
    fn test_wire_serializable_empty_payload() {
        let envelope = Envelope::new(0xAB, vec![]);
        let bytes = encode_envelope(&envelope);

        let result = TestState::from_bytes(&bytes);
        assert!(matches!(result, Err(ProtocolError::EmptyPayload)));
    }
}
