# App Protocol

## Overview

The app protocol defines the wire format for communication between app backends (WASM) and UI surfaces (React).

## Wire Format

All messages use this envelope:

```
┌─────────┬──────────┬─────────────┬─────────────────────┐
│ version │ type_tag │ payload_len │       payload       │
│  (u8)   │   (u8)   │    (u16)    │      (bytes)        │
└─────────┴──────────┴─────────────┴─────────────────────┘
   1 byte    1 byte     2 bytes      0-65535 bytes
```

### Fields

| Field | Size | Description |
|-------|------|-------------|
| version | 1 byte | Protocol version (currently 1) |
| type_tag | 1 byte | Payload type identifier |
| payload_len | 2 bytes | Length of payload (little-endian) |
| payload | 0-65535 bytes | Type-specific data |

### Encoding

```rust
pub fn encode_envelope(type_tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + payload.len());
    buf.push(PROTOCOL_VERSION);  // 1
    buf.push(type_tag);
    buf.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    buf.extend_from_slice(payload);
    buf
}
```

### Decoding

```rust
pub fn decode_envelope(data: &[u8]) -> Result<Envelope, DecodeError> {
    if data.len() < 4 {
        return Err(DecodeError::TooShort);
    }
    
    let version = data[0];
    if version != PROTOCOL_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    
    let type_tag = data[1];
    let payload_len = u16::from_le_bytes([data[2], data[3]]) as usize;
    
    if data.len() < 4 + payload_len {
        return Err(DecodeError::TooShort);
    }
    
    Ok(Envelope {
        type_tag,
        payload: data[4..4 + payload_len].to_vec(),
    })
}
```

## State Types

### Clock State (0x01)

```rust
pub struct ClockState {
    /// Current time (milliseconds since Unix epoch)
    pub timestamp_ms: u64,
    /// Timezone offset (minutes from UTC)
    pub timezone_offset: i16,
}
```

Wire format:
```
┌──────────────────┬───────────────────┐
│   timestamp_ms   │  timezone_offset  │
│     (u64 LE)     │     (i16 LE)      │
└──────────────────┴───────────────────┘
       8 bytes           2 bytes
```

### Calculator State (0x02)

```rust
pub struct CalculatorState {
    /// Current display value
    pub display: String,
    /// Operation indicator (if pending)
    pub operation: Option<char>,
    /// Error state
    pub error: bool,
}
```

Wire format:
```
┌─────────┬───────────┬─────────────┬───────────────┐
│  error  │ operation │ display_len │    display    │
│  (u8)   │   (u8)    │   (u16 LE)  │   (UTF-8)     │
└─────────┴───────────┴─────────────┴───────────────┘
  1 byte    1 byte       2 bytes      variable
```

### Terminal State (0x10)

```rust
pub struct TerminalState {
    /// Terminal output buffer
    pub output: String,
    /// Cursor position
    pub cursor_pos: u32,
    /// Input line (if editing)
    pub input_line: String,
}
```

Wire format:
```
┌────────────┬───────────────┬────────────┬───────────────┬─────────────────┐
│ cursor_pos │  output_len   │   output   │  input_len    │     input       │
│  (u32 LE)  │   (u16 LE)    │  (UTF-8)   │  (u16 LE)     │    (UTF-8)      │
└────────────┴───────────────┴────────────┴───────────────┴─────────────────┘
   4 bytes       2 bytes      variable      2 bytes        variable
```

## Input Types

### Button Press (0x10)

```rust
pub struct ButtonPress {
    pub button_id: String,
}
```

Wire format:
```
┌──────────────┬──────────────┐
│   id_len     │   button_id  │
│   (u8)       │   (UTF-8)    │
└──────────────┴──────────────┘
   1 byte        variable
```

### Text Input (0x11)

```rust
pub struct TextInput {
    pub field_id: String,
    pub text: String,
}
```

### Key Press (0x12)

```rust
pub struct KeyPress {
    /// Key code (e.g., "Enter", "Escape", "a")
    pub key: String,
    /// Modifier keys
    pub modifiers: KeyModifiers,
}

pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}
```

Wire format:
```
┌───────────┬──────────┬───────────┐
│ modifiers │ key_len  │    key    │
│   (u8)    │   (u8)   │  (UTF-8)  │
└───────────┴──────────┴───────────┘
   1 byte     1 byte    variable

Modifiers byte:
  bit 0: ctrl
  bit 1: alt
  bit 2: shift
  bit 3: meta
```

### Terminal Input (0x20)

```rust
pub struct TerminalInput {
    pub action: InputAction,
}

pub enum InputAction {
    /// Raw text input (typing)
    Text(String),
    /// Special key (Enter, Backspace, etc.)
    Key(String),
    /// Command submission (Enter with input)
    Submit(String),
}
```

## Message Flow Example

### Clock App

```
1. UI mounts ClockApp component
   │
   ▼
2. UI → App: MSG_UI_READY
   │
   ▼
3. App starts sending state updates (every 100ms)
   │
   ▼
4. App → UI: MSG_APP_STATE
   ├── type_tag: 0x01 (Clock)
   └── payload: ClockState { timestamp_ms, timezone_offset }
   │
   ▼
5. UI renders current time
```

### Calculator App

```
1. User clicks "5" button
   │
   ▼
2. UI → App: MSG_APP_INPUT
   ├── type_tag: 0x10 (ButtonPress)
   └── payload: { button_id: "5" }
   │
   ▼
3. App updates internal state
   │
   ▼
4. App → UI: MSG_APP_STATE
   ├── type_tag: 0x02 (Calculator)
   └── payload: CalculatorState { display: "5", ... }
   │
   ▼
5. UI renders updated display
```

## Compliance Checklist

### Source Files
- `crates/zos-apps/src/app_protocol/mod.rs` - Protocol exports
- `crates/zos-apps/src/app_protocol/wire.rs` - Envelope encoding
- `crates/zos-apps/src/app_protocol/clock.rs` - Clock state
- `crates/zos-apps/src/app_protocol/calculator.rs` - Calculator state
- `crates/zos-apps/src/app_protocol/terminal.rs` - Terminal state
- `crates/zos-apps/src/app_protocol/input.rs` - Input events

### Key Invariants
- [ ] Protocol version is checked on decode
- [ ] Payload length matches actual data
- [ ] UTF-8 strings are valid
- [ ] Unknown type tags are rejected

### Differences from v0.1.0
- Added protocol version byte
- Terminal state separated from calculator
- KeyPress includes modifier keys
- Envelope helper functions added
