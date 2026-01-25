//! Wire Protocol
//!
//! Binary encoding/decoding for communication between app backends (WASM)
//! and UI surfaces (React).
//!
//! # Wire Format
//!
//! All messages use this envelope:
//!
//! ```text
//! ┌─────────┬──────────┬─────────────┬─────────────────────┐
//! │ version │ type_tag │ payload_len │       payload       │
//! │  (u8)   │   (u8)   │    (u16)    │      (bytes)        │
//! └─────────┴──────────┴─────────────┴─────────────────────┘
//!    1 byte    1 byte     2 bytes      0-65535 bytes
//! ```

mod input;
mod serializable;
mod wire_format;

pub use input::InputEvent;
pub use serializable::WireSerializable;
pub use wire_format::{
    decode_envelope, decode_optional_char, decode_string, decode_u16, decode_u32, decode_u8,
    encode_envelope, encode_optional_char, encode_string, Envelope, PROTOCOL_VERSION,
};

/// Message tags for Backend ↔ UI communication.
///
/// Re-exported from zos-ipc for single source of truth (Invariant 32).
pub mod tags {
    pub use zos_ipc::app::*;
}

/// Type tags for payload identification
pub mod type_tags {
    // State type tags
    pub const TYPE_CLOCK_STATE: u8 = 0x01;
    pub const TYPE_CALCULATOR_STATE: u8 = 0x02;
    pub const TYPE_SETTINGS_STATE: u8 = 0x03;

    // Input type tags
    pub const TYPE_BUTTON_PRESS: u8 = 0x10;
    pub const TYPE_TEXT_INPUT: u8 = 0x11;
    pub const TYPE_KEY_PRESS: u8 = 0x12;
    pub const TYPE_FOCUS_CHANGE: u8 = 0x13;
}

/// Modifier key flags for input events
pub mod modifiers {
    pub const SHIFT: u8 = 1;
    pub const CTRL: u8 = 2;
    pub const ALT: u8 = 4;
}
