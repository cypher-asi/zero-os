//! Settings State Protocol
//!
//! Serialization for Settings app state.

use super::type_tags::TYPE_SETTINGS_STATE;
use super::wire::{decode_string, decode_u8, encode_string, Envelope};
use crate::error::ProtocolError;
use alloc::string::String;
use alloc::vec::Vec;

/// Settings area/section
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum SettingsArea {
    #[default]
    General = 0,
    Identity = 1,
    Permissions = 2,
    Theme = 3,
}

impl SettingsArea {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => SettingsArea::General,
            1 => SettingsArea::Identity,
            2 => SettingsArea::Permissions,
            3 => SettingsArea::Theme,
            _ => SettingsArea::General,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SettingsArea::General => "general",
            SettingsArea::Identity => "identity",
            SettingsArea::Permissions => "permissions",
            SettingsArea::Theme => "theme",
        }
    }
}

/// Settings app state - sent via MSG_APP_STATE
#[derive(Clone, Debug, Default)]
pub struct SettingsState {
    /// Currently active area (0=General, 1=Identity, 2=Permissions, 3=Theme)
    pub active_area: u8,

    /// Current drill-down item (for breadcrumbs), empty if at top level
    pub active_item: String,

    // General settings
    /// Whether to use 24-hour time format
    pub time_format_24h: bool,

    /// Timezone string (e.g., "UTC", "America/New_York")
    pub timezone: String,

    // Theme settings
    /// Theme mode: "dark", "light", "system"
    pub theme: String,

    /// Accent color: "cyan", "blue", "purple", etc.
    pub accent: String,

    /// Background style: "grain", "dots", "waves", etc.
    pub background: String,

    // Identity summary
    /// Whether the user has a neural key set up
    pub has_neural_key: bool,

    /// Number of machine keys registered
    pub machine_key_count: u8,

    /// Number of linked accounts
    pub linked_account_count: u8,

    // Permissions summary
    /// Number of currently running processes
    pub running_process_count: u8,

    /// Total number of capabilities granted
    pub total_capability_count: u8,
}

impl SettingsState {
    /// Create a new SettingsState with defaults
    pub fn new() -> Self {
        Self {
            active_area: 0,
            active_item: String::new(),
            time_format_24h: false,
            timezone: String::from("UTC"),
            theme: String::from("dark"),
            accent: String::from("cyan"),
            background: String::from("grain"),
            has_neural_key: false,
            machine_key_count: 0,
            linked_account_count: 0,
            running_process_count: 0,
            total_capability_count: 0,
        }
    }

    /// Create initial state
    pub fn initial() -> Self {
        Self::new()
    }

    /// Serialize to bytes (for sending via IPC)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut payload = Vec::new();

        // Type tag
        payload.push(TYPE_SETTINGS_STATE);

        // active_area (u8)
        payload.push(self.active_area);

        // active_item (length-prefixed string)
        payload.extend_from_slice(&encode_string(&self.active_item));

        // General settings
        payload.push(if self.time_format_24h { 1 } else { 0 });
        payload.extend_from_slice(&encode_string(&self.timezone));

        // Theme settings
        payload.extend_from_slice(&encode_string(&self.theme));
        payload.extend_from_slice(&encode_string(&self.accent));
        payload.extend_from_slice(&encode_string(&self.background));

        // Identity summary
        payload.push(if self.has_neural_key { 1 } else { 0 });
        payload.push(self.machine_key_count);
        payload.push(self.linked_account_count);

        // Permissions summary
        payload.push(self.running_process_count);
        payload.push(self.total_capability_count);

        // Wrap in envelope
        let envelope = Envelope::new(TYPE_SETTINGS_STATE, payload);
        super::wire::encode_envelope(&envelope)
    }

    /// Deserialize from bytes (received via IPC)
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProtocolError> {
        // Decode envelope
        let envelope = super::wire::decode_envelope(data)?;

        // Check type tag
        if envelope.type_tag != TYPE_SETTINGS_STATE {
            return Err(ProtocolError::UnexpectedType {
                expected: TYPE_SETTINGS_STATE,
                got: envelope.type_tag,
            });
        }

        let payload = &envelope.payload;
        if payload.is_empty() {
            return Err(ProtocolError::EmptyPayload);
        }

        // Skip type tag in payload (already checked via envelope)
        let mut cursor = 1;

        // Parse fields
        let active_area = decode_u8(payload, &mut cursor)?;
        let active_item = decode_string(payload, &mut cursor)?;

        // General settings
        let time_format_24h = decode_u8(payload, &mut cursor)? != 0;
        let timezone = decode_string(payload, &mut cursor)?;

        // Theme settings
        let theme = decode_string(payload, &mut cursor)?;
        let accent = decode_string(payload, &mut cursor)?;
        let background = decode_string(payload, &mut cursor)?;

        // Identity summary
        let has_neural_key = decode_u8(payload, &mut cursor)? != 0;
        let machine_key_count = decode_u8(payload, &mut cursor)?;
        let linked_account_count = decode_u8(payload, &mut cursor)?;

        // Permissions summary
        let running_process_count = decode_u8(payload, &mut cursor)?;
        let total_capability_count = decode_u8(payload, &mut cursor)?;

        Ok(SettingsState {
            active_area,
            active_item,
            time_format_24h,
            timezone,
            theme,
            accent,
            background,
            has_neural_key,
            machine_key_count,
            linked_account_count,
            running_process_count,
            total_capability_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_state_roundtrip() {
        let state = SettingsState {
            active_area: 2,
            active_item: String::from("permissions"),
            time_format_24h: true,
            timezone: String::from("America/New_York"),
            theme: String::from("dark"),
            accent: String::from("cyan"),
            background: String::from("grain"),
            has_neural_key: true,
            machine_key_count: 3,
            linked_account_count: 2,
            running_process_count: 5,
            total_capability_count: 12,
        };

        let bytes = state.to_bytes();
        let decoded = SettingsState::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.active_area, state.active_area);
        assert_eq!(decoded.active_item, state.active_item);
        assert_eq!(decoded.time_format_24h, state.time_format_24h);
        assert_eq!(decoded.timezone, state.timezone);
        assert_eq!(decoded.theme, state.theme);
        assert_eq!(decoded.accent, state.accent);
        assert_eq!(decoded.background, state.background);
        assert_eq!(decoded.has_neural_key, state.has_neural_key);
        assert_eq!(decoded.machine_key_count, state.machine_key_count);
        assert_eq!(decoded.linked_account_count, state.linked_account_count);
        assert_eq!(decoded.running_process_count, state.running_process_count);
        assert_eq!(decoded.total_capability_count, state.total_capability_count);
    }

    #[test]
    fn test_settings_state_initial() {
        let state = SettingsState::initial();

        let bytes = state.to_bytes();
        let decoded = SettingsState::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.active_area, 0);
        assert_eq!(decoded.theme, "dark");
        assert_eq!(decoded.accent, "cyan");
    }

    #[test]
    fn test_settings_area_conversion() {
        assert_eq!(SettingsArea::from_u8(0), SettingsArea::General);
        assert_eq!(SettingsArea::from_u8(1), SettingsArea::Identity);
        assert_eq!(SettingsArea::from_u8(2), SettingsArea::Permissions);
        assert_eq!(SettingsArea::from_u8(3), SettingsArea::Theme);
        assert_eq!(SettingsArea::from_u8(99), SettingsArea::General); // Invalid defaults to General
    }
}
