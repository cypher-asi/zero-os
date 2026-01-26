//! Authorization module for Identity Service
//!
//! Implements permission checks per zos-service.md Rule 4 (FAIL-CLOSED).
//!
//! # Safety Invariants (per zos-service.md Rule 0)
//!
//! ## Success Conditions
//! - Authorization check succeeds only when:
//!   1. Caller PID is in the trusted process list, OR
//!   2. (Future) Caller presents a valid session token for the target user
//!
//! ## Forbidden States
//! - Allowing access without explicit authorization check
//! - Implicit trust based on caller claims alone
//! - Returning Allowed for unknown/unverified callers
//!
//! # Security Model
//!
//! ## Trusted Processes (Phase 1 - Current)
//! - **System processes** (PID 1-9): Kernel-level services with full access
//! - **Desktop process** (PID 10): Trusted user-facing application
//!
//! ## Future Enhancement (Phase 2)
//! - Session token verification via Permission Service
//! - Capability-based authorization for fine-grained access control
//!
//! # Authorization Flow
//!
//! 1. Check if caller PID is in TRUSTED_SYSTEM_PIDS → ALLOW
//! 2. Check if caller PID is DESKTOP_PROCESS_PID → ALLOW (with session validation TODO)
//! 3. (Future) Verify session token with Permission Service
//! 4. On ANY uncertainty → DENY (fail-closed)

use alloc::format;
use zos_apps::syscall;

// =============================================================================
// Trusted Process Configuration
// =============================================================================

/// System processes with implicit trust (kernel-level services).
/// These PIDs are assigned during boot and never change.
///
/// - PID 0: Init process (not used for IPC)
/// - PID 1: Init service handler
/// - PID 2: Permission service
/// - PID 3: Identity service (this service)
/// - PID 4: VFS service
/// - PID 5: Time service
/// - PIDs 6-9: Reserved for future system services
const TRUSTED_SYSTEM_PID_MAX: u32 = 9;

/// The desktop process PID - the trusted user-facing application.
/// Desktop mediates all user interactions and validates user sessions.
///
/// NOTE: In Phase 2, this will require session token verification
/// instead of implicit PID trust.
const DESKTOP_PROCESS_PID: u32 = 10;

// =============================================================================
// Authorization Types
// =============================================================================

/// Result of an authorization check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthResult {
    /// Caller is authorized to perform the operation
    Allowed,
    /// Caller is NOT authorized - request must be denied
    Denied,
}

/// Reason for allowing or denying authorization (for logging/debugging).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthReason {
    /// Allowed: Caller is a system process (PID < 10)
    SystemProcess,
    /// Allowed: Caller is the desktop process (PID 10)
    DesktopProcess,
    /// Allowed: Session token verified (future)
    #[allow(dead_code)]
    SessionVerified,
    /// Denied: Unknown/untrusted process
    UntrustedProcess,
    /// Denied: Session token invalid or missing (future)
    #[allow(dead_code)]
    InvalidSession,
}

// =============================================================================
// Authorization Functions
// =============================================================================

/// Check if a process is authorized to perform operations on a user's identity.
///
/// # Rule 4 Compliance (FAIL-CLOSED)
///
/// This function implements fail-closed authorization:
/// - If authorization cannot be determined → DENY
/// - System processes (PID <= TRUSTED_SYSTEM_PID_MAX) have implicit trust
/// - Desktop process (PID == DESKTOP_PROCESS_PID) is trusted
/// - All other processes are denied by default
///
/// # Arguments
///
/// * `from_pid` - The PID of the process making the request
/// * `target_user_id` - The user_id the operation will affect
///
/// # Returns
///
/// * `AuthResult::Allowed` - Proceed with the operation
/// * `AuthResult::Denied` - Reject the request with Unauthorized error
pub fn check_user_authorization(from_pid: u32, target_user_id: u128) -> AuthResult {
    let (result, reason) = check_authorization_with_reason(from_pid, target_user_id);

    // Log the decision with reason
    match result {
        AuthResult::Allowed => {
            syscall::debug(&format!(
                "IdentityService: Auth ALLOW - PID {} for user {:032x} (reason: {:?})",
                from_pid, target_user_id, reason
            ));
        }
        AuthResult::Denied => {
            syscall::debug(&format!(
                "IdentityService: Auth DENY - PID {} for user {:032x} (reason: {:?})",
                from_pid, target_user_id, reason
            ));
        }
    }

    result
}

/// Check authorization with detailed reason (for internal use and testing).
fn check_authorization_with_reason(from_pid: u32, _target_user_id: u128) -> (AuthResult, AuthReason) {
    // Check 1: System processes have full access
    // These are trusted kernel-level services with assigned PIDs
    if from_pid <= TRUSTED_SYSTEM_PID_MAX {
        return (AuthResult::Allowed, AuthReason::SystemProcess);
    }

    // Check 2: Desktop process is trusted to act on behalf of users
    // NOTE: In Phase 2, this should verify session token instead of trusting PID
    // TODO: Implement session token verification:
    //   1. Desktop should include session_token in request
    //   2. Verify token with Permission Service
    //   3. Extract user_id from token and compare with target_user_id
    if from_pid == DESKTOP_PROCESS_PID {
        return (AuthResult::Allowed, AuthReason::DesktopProcess);
    }

    // Check 3: (Future) Verify session token
    // This would involve:
    //   - Extracting session token from request
    //   - Verifying token signature
    //   - Checking token hasn't expired
    //   - Verifying token's user_id matches target_user_id

    // FAIL-CLOSED: All other processes are denied by default
    // This is a security-critical decision per Rule 4
    (AuthResult::Denied, AuthReason::UntrustedProcess)
}

/// Log a permission denial (Rule 10: Log permission denials explicitly)
pub fn log_denial(operation: &str, from_pid: u32, target_user_id: u128) {
    syscall::debug(&format!(
        "IdentityService: PERMISSION_DENIED op={} from_pid={} target_user={:032x}",
        operation, from_pid, target_user_id
    ));
}

/// Check if a PID is a system process (for external use).
#[inline]
pub fn is_system_process(pid: u32) -> bool {
    pid <= TRUSTED_SYSTEM_PID_MAX
}

/// Check if a PID is the desktop process (for external use).
#[inline]
pub fn is_desktop_process(pid: u32) -> bool {
    pid == DESKTOP_PROCESS_PID
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // System process authorization tests
    // =========================================================================

    #[test]
    fn test_system_process_allowed() {
        // System processes (PID <= 9) should always be allowed
        assert_eq!(check_user_authorization(0, 12345), AuthResult::Allowed);
        assert_eq!(check_user_authorization(1, 12345), AuthResult::Allowed);
        assert_eq!(check_user_authorization(3, 12345), AuthResult::Allowed);
        assert_eq!(check_user_authorization(9, 12345), AuthResult::Allowed);
    }

    #[test]
    fn test_system_process_reason() {
        let (result, reason) = check_authorization_with_reason(5, 12345);
        assert_eq!(result, AuthResult::Allowed);
        assert_eq!(reason, AuthReason::SystemProcess);
    }

    // =========================================================================
    // Desktop process authorization tests
    // =========================================================================

    #[test]
    fn test_desktop_process_allowed() {
        // Desktop (PID 10) should be allowed
        assert_eq!(check_user_authorization(10, 12345), AuthResult::Allowed);
    }

    #[test]
    fn test_desktop_process_reason() {
        let (result, reason) = check_authorization_with_reason(10, 12345);
        assert_eq!(result, AuthResult::Allowed);
        assert_eq!(reason, AuthReason::DesktopProcess);
    }

    // =========================================================================
    // Unknown process denial tests (FAIL-CLOSED)
    // =========================================================================

    #[test]
    fn test_unknown_process_denied() {
        // Unknown processes should be denied (fail-closed per Rule 4)
        assert_eq!(check_user_authorization(11, 12345), AuthResult::Denied);
        assert_eq!(check_user_authorization(50, 12345), AuthResult::Denied);
        assert_eq!(check_user_authorization(100, 12345), AuthResult::Denied);
    }

    #[test]
    fn test_unknown_process_reason() {
        let (result, reason) = check_authorization_with_reason(99, 12345);
        assert_eq!(result, AuthResult::Denied);
        assert_eq!(reason, AuthReason::UntrustedProcess);
    }

    // =========================================================================
    // Helper function tests
    // =========================================================================

    #[test]
    fn test_is_system_process() {
        assert!(is_system_process(0));
        assert!(is_system_process(5));
        assert!(is_system_process(9));
        assert!(!is_system_process(10));
        assert!(!is_system_process(11));
    }

    #[test]
    fn test_is_desktop_process() {
        assert!(!is_desktop_process(9));
        assert!(is_desktop_process(10));
        assert!(!is_desktop_process(11));
    }

    // =========================================================================
    // Boundary tests
    // =========================================================================

    #[test]
    fn test_boundary_between_system_and_desktop() {
        // PID 9 is last system process
        let (result9, reason9) = check_authorization_with_reason(9, 12345);
        assert_eq!(result9, AuthResult::Allowed);
        assert_eq!(reason9, AuthReason::SystemProcess);

        // PID 10 is desktop
        let (result10, reason10) = check_authorization_with_reason(10, 12345);
        assert_eq!(result10, AuthResult::Allowed);
        assert_eq!(reason10, AuthReason::DesktopProcess);

        // PID 11 is first untrusted
        let (result11, reason11) = check_authorization_with_reason(11, 12345);
        assert_eq!(result11, AuthResult::Denied);
        assert_eq!(reason11, AuthReason::UntrustedProcess);
    }

    #[test]
    fn test_authorization_different_users() {
        // Same PID should have same result regardless of target user
        // (until we implement session token verification)
        assert_eq!(check_user_authorization(10, 0), AuthResult::Allowed);
        assert_eq!(check_user_authorization(10, 12345), AuthResult::Allowed);
        assert_eq!(check_user_authorization(10, u128::MAX), AuthResult::Allowed);
    }
}
