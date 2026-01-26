/**
 * App Protocol - Type Definitions
 *
 * Constants and type definitions for the app protocol.
 */

// Protocol version
export const PROTOCOL_VERSION = 0x01;

// Message tags for IPC communication
export const MSG_APP_STATE = 0x2000;
export const MSG_APP_INPUT = 0x2001;
export const MSG_UI_READY = 0x2002;
export const MSG_APP_FOCUS = 0x2003;
export const MSG_APP_ERROR = 0x2004;

// Capability revocation notification (supervisor -> process)
// Payload: [slot: u32, object_type: u8, object_id: u64, reason: u8]
export const MSG_CAP_REVOKED = 0x3010;

// Revocation reasons
export const REVOKE_REASON_EXPLICIT = 1; // Supervisor/user revoked
export const REVOKE_REASON_EXPIRED = 2; // Capability expired
export const REVOKE_REASON_PROCESS_EXIT = 3; // Source process exited

// Type tags for payload identification
export const TYPE_CLOCK_STATE = 0x01;
export const TYPE_CALCULATOR_STATE = 0x02;
export const TYPE_SETTINGS_STATE = 0x03;
export const TYPE_BUTTON_PRESS = 0x10;
export const TYPE_TEXT_INPUT = 0x11;
export const TYPE_KEY_PRESS = 0x12;
export const TYPE_FOCUS_CHANGE = 0x13;

// Permission type tags
export const TYPE_GRANT_REQUEST = 0x20;
export const TYPE_REVOKE_REQUEST = 0x21;
export const TYPE_PERMISSION_RESPONSE = 0x22;
export const TYPE_PERMISSION_LIST = 0x23;

// Permission message tags
export const MSG_GRANT_PERMISSION = 0x1010;
export const MSG_REVOKE_PERMISSION = 0x1011;
export const MSG_LIST_PERMISSIONS = 0x1012;
export const MSG_PERMISSION_RESPONSE = 0x1013;

// Modifier key constants
export const MODIFIER_SHIFT = 1;
export const MODIFIER_CTRL = 2;
export const MODIFIER_ALT = 4;
