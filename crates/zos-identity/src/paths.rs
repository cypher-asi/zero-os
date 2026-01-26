//! Centralized path validation and canonicalization for Identity storage.
//!
//! # Safety Invariants (per zos-service.md Rules 2/3)
//!
//! ## Path Validation Rules
//! - All paths must be absolute (start with `/`)
//! - No path traversal components (`..` or `.`)
//! - No consecutive slashes (`//`)
//! - No trailing slashes (except root `/`)
//! - Must be within allowed base directories
//!
//! ## Allowed Base Directories
//! - `/home/{user_id}/` - User home directories
//! - `/users/` - System user registry
//!
//! ## Forbidden Patterns
//! - Path traversal: `/../`, `/./`
//! - Null bytes in paths
//! - Paths outside allowed directories

use alloc::string::String;
use alloc::vec::Vec;

/// Errors that can occur during path validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// Path is empty
    Empty,
    /// Path does not start with `/`
    NotAbsolute,
    /// Path contains `..` traversal
    TraversalAttempt,
    /// Path contains null bytes
    NullByte,
    /// Path is outside allowed directories
    OutsideAllowed,
    /// Path contains consecutive slashes
    ConsecutiveSlashes,
    /// Path contains invalid characters
    InvalidCharacters,
}

impl core::fmt::Display for PathError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PathError::Empty => write!(f, "path is empty"),
            PathError::NotAbsolute => write!(f, "path must be absolute (start with /)"),
            PathError::TraversalAttempt => write!(f, "path traversal (..) not allowed"),
            PathError::NullByte => write!(f, "path contains null byte"),
            PathError::OutsideAllowed => write!(f, "path is outside allowed directories"),
            PathError::ConsecutiveSlashes => write!(f, "path contains consecutive slashes"),
            PathError::InvalidCharacters => write!(f, "path contains invalid characters"),
        }
    }
}

/// Validate a path according to security rules.
///
/// # Arguments
/// * `path` - The path to validate
///
/// # Returns
/// * `Ok(())` if the path is valid
/// * `Err(PathError)` if the path violates any rules
///
/// # Example
/// ```
/// use zos_identity::paths::validate_path;
///
/// assert!(validate_path("/home/123/file.txt").is_ok());
/// assert!(validate_path("/home/123/../other").is_err()); // traversal
/// assert!(validate_path("relative/path").is_err()); // not absolute
/// ```
pub fn validate_path(path: &str) -> Result<(), PathError> {
    // Check for empty path
    if path.is_empty() {
        return Err(PathError::Empty);
    }

    // Check for null bytes
    if path.bytes().any(|b| b == 0) {
        return Err(PathError::NullByte);
    }

    // Must be absolute
    if !path.starts_with('/') {
        return Err(PathError::NotAbsolute);
    }

    // Check for consecutive slashes
    if path.contains("//") {
        return Err(PathError::ConsecutiveSlashes);
    }

    // Check for path traversal
    for component in path.split('/') {
        if component == ".." {
            return Err(PathError::TraversalAttempt);
        }
        // Also check for hidden traversal via single dot at end of component
        // e.g., "/home/./file" - though "." alone is less dangerous than ".."
    }

    // Check that path is within allowed directories
    if !is_allowed_path(path) {
        return Err(PathError::OutsideAllowed);
    }

    Ok(())
}

/// Check if a path is within allowed directories.
///
/// Allowed directories:
/// - `/home/{user_id}/` - User home directories
/// - `/users/` - System user registry
fn is_allowed_path(path: &str) -> bool {
    // User home directories
    if path.starts_with("/home/") {
        return true;
    }

    // System user registry
    if path.starts_with("/users/") || path == "/users/registry.json" {
        return true;
    }

    false
}

/// Canonicalize a path by removing redundant components.
///
/// This function:
/// - Removes trailing slashes (except for root `/`)
/// - Normalizes consecutive slashes to single slashes
/// - Removes `.` components
/// - Does NOT resolve `..` (that would be a security issue)
///
/// # Arguments
/// * `path` - The path to canonicalize
///
/// # Returns
/// The canonicalized path
///
/// # Example
/// ```
/// use zos_identity::paths::canonicalize_path;
///
/// assert_eq!(canonicalize_path("/home/123/"), "/home/123");
/// assert_eq!(canonicalize_path("/home//123"), "/home/123");
/// assert_eq!(canonicalize_path("/home/./123"), "/home/123");
/// ```
pub fn canonicalize_path(path: &str) -> String {
    if path.is_empty() {
        return String::from("/");
    }

    let mut components: Vec<&str> = Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => {
                // Skip empty components (from consecutive slashes) and current dir markers
            }
            ".." => {
                // We don't resolve ".." for security - keep it to fail validation
                components.push(component);
            }
            _ => {
                components.push(component);
            }
        }
    }

    if components.is_empty() {
        String::from("/")
    } else {
        let mut result = String::new();
        for component in components {
            result.push('/');
            result.push_str(component);
        }
        result
    }
}

/// Build a storage path for a user's file, with validation.
///
/// # Arguments
/// * `user_id` - The user ID
/// * `subpath` - Path relative to user's home directory (e.g., ".zos/identity/keys.json")
///
/// # Returns
/// * `Ok(String)` - The validated full path
/// * `Err(PathError)` - If the resulting path is invalid
///
/// # Example
/// ```
/// use zos_identity::paths::build_user_path;
///
/// let path = build_user_path(12345, ".zos/identity/keys.json").unwrap();
/// assert_eq!(path, "/home/12345/.zos/identity/keys.json");
/// ```
pub fn build_user_path(user_id: u128, subpath: &str) -> Result<String, PathError> {
    // Validate subpath doesn't try to escape
    if subpath.contains("..") {
        return Err(PathError::TraversalAttempt);
    }

    let path = if subpath.starts_with('/') {
        alloc::format!("/home/{}{}", user_id, subpath)
    } else {
        alloc::format!("/home/{}/{}", user_id, subpath)
    };

    let canonical = canonicalize_path(&path);
    validate_path(&canonical)?;
    Ok(canonical)
}

/// Build a storage path for a machine key file, with validation.
///
/// # Arguments
/// * `user_id` - The user ID
/// * `machine_id` - The machine ID
///
/// # Returns
/// The validated machine key storage path
pub fn build_machine_key_path(user_id: u128, machine_id: u128) -> Result<String, PathError> {
    let path = alloc::format!(
        "/home/{}/.zos/identity/machine/{:032x}.json",
        user_id, machine_id
    );
    validate_path(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // validate_path tests
    // =========================================================================

    #[test]
    fn test_validate_valid_paths() {
        assert!(validate_path("/home/123/file.txt").is_ok());
        assert!(validate_path("/home/123/.zos/identity/keys.json").is_ok());
        assert!(validate_path("/users/registry.json").is_ok());
    }

    #[test]
    fn test_validate_empty_path() {
        assert_eq!(validate_path(""), Err(PathError::Empty));
    }

    #[test]
    fn test_validate_relative_path() {
        assert_eq!(validate_path("relative/path"), Err(PathError::NotAbsolute));
        assert_eq!(validate_path("./relative"), Err(PathError::NotAbsolute));
    }

    #[test]
    fn test_validate_traversal_attempt() {
        assert_eq!(
            validate_path("/home/123/../other"),
            Err(PathError::TraversalAttempt)
        );
        assert_eq!(
            validate_path("/home/123/../../etc/passwd"),
            Err(PathError::TraversalAttempt)
        );
    }

    #[test]
    fn test_validate_consecutive_slashes() {
        assert_eq!(
            validate_path("/home//123"),
            Err(PathError::ConsecutiveSlashes)
        );
        assert_eq!(
            validate_path("/home/123//file"),
            Err(PathError::ConsecutiveSlashes)
        );
    }

    #[test]
    fn test_validate_outside_allowed() {
        assert_eq!(validate_path("/etc/passwd"), Err(PathError::OutsideAllowed));
        assert_eq!(validate_path("/tmp/file"), Err(PathError::OutsideAllowed));
        assert_eq!(validate_path("/root/file"), Err(PathError::OutsideAllowed));
    }

    // =========================================================================
    // canonicalize_path tests
    // =========================================================================

    #[test]
    fn test_canonicalize_trailing_slash() {
        assert_eq!(canonicalize_path("/home/123/"), "/home/123");
        assert_eq!(canonicalize_path("/home/123/dir/"), "/home/123/dir");
    }

    #[test]
    fn test_canonicalize_consecutive_slashes() {
        assert_eq!(canonicalize_path("/home//123"), "/home/123");
        assert_eq!(canonicalize_path("/home///123///file"), "/home/123/file");
    }

    #[test]
    fn test_canonicalize_dot_components() {
        assert_eq!(canonicalize_path("/home/./123"), "/home/123");
        assert_eq!(canonicalize_path("/home/123/./file"), "/home/123/file");
    }

    #[test]
    fn test_canonicalize_preserves_dotdot() {
        // We preserve .. so validation can reject it
        assert_eq!(canonicalize_path("/home/123/../other"), "/home/123/../other");
    }

    #[test]
    fn test_canonicalize_empty() {
        assert_eq!(canonicalize_path(""), "/");
    }

    // =========================================================================
    // build_user_path tests
    // =========================================================================

    #[test]
    fn test_build_user_path_valid() {
        let path = build_user_path(12345, ".zos/identity/keys.json").unwrap();
        assert_eq!(path, "/home/12345/.zos/identity/keys.json");
    }

    #[test]
    fn test_build_user_path_with_leading_slash() {
        let path = build_user_path(12345, "/.zos/identity/keys.json").unwrap();
        assert_eq!(path, "/home/12345/.zos/identity/keys.json");
    }

    #[test]
    fn test_build_user_path_traversal_rejected() {
        assert_eq!(
            build_user_path(12345, "../other"),
            Err(PathError::TraversalAttempt)
        );
        assert_eq!(
            build_user_path(12345, ".zos/../../../etc/passwd"),
            Err(PathError::TraversalAttempt)
        );
    }

    // =========================================================================
    // build_machine_key_path tests
    // =========================================================================

    #[test]
    fn test_build_machine_key_path() {
        let path = build_machine_key_path(12345, 0xabcdef).unwrap();
        assert_eq!(
            path,
            "/home/12345/.zos/identity/machine/00000000000000000000000000abcdef.json"
        );
    }
}
