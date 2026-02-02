//! Identity Service message handlers
//!
//! Organized by functional domain:
//! - `keys`: Neural key and machine key operations
//! - `session`: ZID login/enrollment flows
//! - `credentials`: Credential management
//! - `preferences`: Identity preferences (default key scheme, etc.)
//!
//! # Safety Invariants (per zos-service.md Rule 0)
//!
//! All handlers in this module MUST:
//!
//! ## Authorization (Rule 4 - FAIL-CLOSED)
//! - Verify caller (msg.from_pid) is authorized to act on request.user_id
//! - DENY if authorization cannot be determined
//! - NEVER trust user-provided identity data without verification
//!
//! ## Input Parsing (Rule 1)
//! - JSON parse failure → KeyError::InvalidRequest (not DerivationFailed)
//! - NEVER proceed after parse failure
//! - NEVER return empty results on parse failure (silent fallthrough forbidden)
//!
//! ## Response Guarantees (Rule 6)
//! - Exactly ONE response per client request
//! - Only final stage may respond
//! - No intermediate responses

pub mod credentials;
pub mod keys;
pub mod preferences;
pub mod registration;
pub mod session;
pub mod tier;
