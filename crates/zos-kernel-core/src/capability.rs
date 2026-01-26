//! Capability-based access control verification
//!
//! This module implements the Axiom layer's capability verification:
//! - Capability tokens with permissions
//! - Capability spaces (per-process)
//! - The `axiom_check` function for authority verification
//!
//! # Security Properties (Verification Targets)
//!
//! 1. **No Forged Object**: Only IDs we inserted can be returned
//! 2. **No Rights Escalation**: axiom_check never returns rights > stored rights
//! 3. **Fail Closed**: Invalid/malformed handles always error

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::types::{CapSlot, ObjectType, Permissions};

/// A capability token - proof of authority to access a resource
#[derive(Clone, Debug)]
pub struct Capability {
    /// Unique capability ID
    pub id: u64,
    /// Type of object this capability references
    pub object_type: ObjectType,
    /// ID of the referenced object
    pub object_id: u64,
    /// Permissions granted by this capability
    pub permissions: Permissions,
    /// Generation number (for revocation tracking)
    pub generation: u32,
    /// Expiration timestamp (nanos since boot, 0 = never expires)
    pub expires_at: u64,
}

impl Capability {
    /// Check if this capability has expired.
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.expires_at != 0 && current_time > self.expires_at
    }

    /// Check if this capability has sufficient permissions for an operation
    pub fn has_permissions(&self, required: &Permissions) -> bool {
        (!required.read || self.permissions.read)
            && (!required.write || self.permissions.write)
            && (!required.grant || self.permissions.grant)
    }
}

/// Per-process capability table
pub struct CapabilitySpace {
    /// Capability slots
    pub slots: BTreeMap<CapSlot, Capability>,
    /// Next slot to allocate
    pub next_slot: CapSlot,
}

impl CapabilitySpace {
    /// Create a new empty capability space
    pub fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            next_slot: 0,
        }
    }

    /// Insert a capability, returning its slot
    pub fn insert(&mut self, cap: Capability) -> CapSlot {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.slots.insert(slot, cap);
        slot
    }

    /// Get a capability by slot
    pub fn get(&self, slot: CapSlot) -> Option<&Capability> {
        self.slots.get(&slot)
    }

    /// Get a mutable capability by slot
    pub fn get_mut(&mut self, slot: CapSlot) -> Option<&mut Capability> {
        self.slots.get_mut(&slot)
    }

    /// Remove a capability
    pub fn remove(&mut self, slot: CapSlot) -> Option<Capability> {
        self.slots.remove(&slot)
    }

    /// List all capabilities
    pub fn list(&self) -> Vec<(CapSlot, Capability)> {
        self.slots.iter().map(|(&s, c)| (s, c.clone())).collect()
    }

    /// Number of capabilities
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Check if a slot exists
    pub fn contains(&self, slot: CapSlot) -> bool {
        self.slots.contains_key(&slot)
    }
}

impl Default for CapabilitySpace {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Axiom Capability Checking - THE verification target
// ============================================================================

/// Errors returned by Axiom capability checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxiomError {
    /// Capability slot is empty or invalid
    InvalidSlot,
    /// Capability references wrong object type
    WrongType,
    /// Capability lacks required permissions
    InsufficientRights,
    /// Capability has expired
    Expired,
    /// Object no longer exists
    ObjectNotFound,
}

/// Check if a process has authority to perform an operation.
///
/// This is the Axiom gatekeeper function. Every syscall that requires
/// authority calls this before executing.
///
/// # Arguments
/// - `cspace`: The process's capability space
/// - `slot`: The capability slot being used
/// - `required`: Minimum permissions needed
/// - `expected_type`: Expected object type (optional)
/// - `current_time`: Current time in nanos for expiration check
///
/// # Returns
/// - `Ok(&Capability)`: Authority granted, reference to the capability
/// - `Err(AxiomError)`: Authority denied with reason
///
/// # Security Properties (Verification Targets)
///
/// 1. **No Forged Object**: The returned capability was actually inserted into cspace
/// 2. **No Rights Escalation**: Returned cap.permissions <= required (checked via has_permissions)
/// 3. **Fail Closed**: Any error condition returns Err, never an invalid capability
///
/// # Invariants
/// - This function never modifies any state
/// - All kernel operations call this before executing
pub fn axiom_check<'a>(
    cspace: &'a CapabilitySpace,
    slot: CapSlot,
    required: &Permissions,
    expected_type: Option<ObjectType>,
    current_time: u64,
) -> Result<&'a Capability, AxiomError> {
    // 1. Lookup capability - FAIL CLOSED on missing slot
    let cap = cspace.get(slot).ok_or(AxiomError::InvalidSlot)?;

    // 2. Check object type (if specified) - FAIL CLOSED on type mismatch
    if let Some(expected) = expected_type {
        if cap.object_type != expected {
            return Err(AxiomError::WrongType);
        }
    }

    // 3. Check permissions - NO RIGHTS ESCALATION
    // The capability must have AT LEAST the required permissions
    if !cap.has_permissions(required) {
        return Err(AxiomError::InsufficientRights);
    }

    // 4. Check expiration - FAIL CLOSED on expired
    if cap.is_expired(current_time) {
        return Err(AxiomError::Expired);
    }

    // All checks passed - NO FORGED OBJECT (we only return what was in cspace)
    Ok(cap)
}

// ============================================================================
// Kani Proofs for Capability Verification
// ============================================================================

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Proof: No forged objects - only IDs we inserted can be returned
    #[kani::proof]
    fn no_forged_object() {
        let mut cspace = CapabilitySpace::new();

        // Insert a capability with known values
        let cap = Capability {
            id: kani::any(),
            object_type: ObjectType::Endpoint,
            object_id: kani::any(),
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let inserted_object_id = cap.object_id;
        let slot = cspace.insert(cap);

        // axiom_check should only return the capability we inserted
        let result = axiom_check(
            &cspace,
            slot,
            &Permissions::read_only(),
            Some(ObjectType::Endpoint),
            0,
        );

        // If successful, the object_id must match what we inserted
        if let Ok(returned_cap) = result {
            kani::assert(
                returned_cap.object_id == inserted_object_id,
                "Returned capability must have the same object_id as inserted",
            );
        }
    }

    /// Proof: No rights escalation - axiom_check never returns rights > stored rights
    #[kani::proof]
    fn no_rights_escalation() {
        let mut cspace = CapabilitySpace::new();

        // Insert a capability with specific permissions
        let cap_perms = Permissions {
            read: kani::any(),
            write: kani::any(),
            grant: kani::any(),
        };
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: cap_perms,
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        // Request permissions
        let required = Permissions {
            read: kani::any(),
            write: kani::any(),
            grant: kani::any(),
        };

        let result = axiom_check(&cspace, slot, &required, None, 0);

        // If check succeeds, returned capability permissions must include all required permissions
        if let Ok(returned_cap) = result {
            // The returned capability must have the permissions we need
            if required.read {
                kani::assert(
                    returned_cap.permissions.read,
                    "If read was required, returned cap must have read",
                );
            }
            if required.write {
                kani::assert(
                    returned_cap.permissions.write,
                    "If write was required, returned cap must have write",
                );
            }
            if required.grant {
                kani::assert(
                    returned_cap.permissions.grant,
                    "If grant was required, returned cap must have grant",
                );
            }
        }
    }

    /// Proof: Fail closed - invalid/malformed handles always error
    #[kani::proof]
    fn fail_closed_invalid_slot() {
        let cspace = CapabilitySpace::new(); // Empty cspace

        let slot: CapSlot = kani::any();

        // Any slot lookup on empty cspace must fail
        let result = axiom_check(&cspace, slot, &Permissions::read_only(), None, 0);

        kani::assert(
            result.is_err(),
            "Empty cspace must always return error for any slot",
        );
    }

    /// Proof: Fail closed - wrong type always errors
    #[kani::proof]
    fn fail_closed_wrong_type() {
        let mut cspace = CapabilitySpace::new();

        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        // Request a different type
        let result = axiom_check(
            &cspace,
            slot,
            &Permissions::read_only(),
            Some(ObjectType::Process),
            0,
        );

        kani::assert(
            matches!(result, Err(AxiomError::WrongType)),
            "Wrong type must always error",
        );
    }

    /// Proof: Fail closed - insufficient rights always errors
    #[kani::proof]
    fn fail_closed_insufficient_rights() {
        let mut cspace = CapabilitySpace::new();

        // Insert capability with read-only
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        // Request write permission (which we don't have)
        let result = axiom_check(&cspace, slot, &Permissions::write_only(), None, 0);

        kani::assert(
            matches!(result, Err(AxiomError::InsufficientRights)),
            "Insufficient rights must always error",
        );
    }

    /// Proof: Fail closed - expired capabilities always error
    #[kani::proof]
    fn fail_closed_expired() {
        let mut cspace = CapabilitySpace::new();

        let expiry: u64 = kani::any();
        kani::assume(expiry > 0 && expiry < u64::MAX); // Non-zero expiry

        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: expiry,
        };
        let slot = cspace.insert(cap);

        let current_time = expiry + 1; // After expiration

        let result = axiom_check(&cspace, slot, &Permissions::read_only(), None, current_time);

        kani::assert(
            matches!(result, Err(AxiomError::Expired)),
            "Expired capabilities must always error",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axiom_check_valid_capability() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        let result = axiom_check(
            &cspace,
            slot,
            &Permissions::read_only(),
            Some(ObjectType::Endpoint),
            0,
        );

        assert!(result.is_ok());
        let cap = result.unwrap();
        assert_eq!(cap.object_id, 42);
    }

    #[test]
    fn test_axiom_check_invalid_slot() {
        let cspace = CapabilitySpace::new();

        let result = axiom_check(&cspace, 999, &Permissions::read_only(), None, 0);

        assert!(matches!(result, Err(AxiomError::InvalidSlot)));
    }

    #[test]
    fn test_axiom_check_wrong_type() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        let result = axiom_check(
            &cspace,
            slot,
            &Permissions::read_only(),
            Some(ObjectType::Process),
            0,
        );

        assert!(matches!(result, Err(AxiomError::WrongType)));
    }

    #[test]
    fn test_axiom_check_insufficient_permissions() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        let result = axiom_check(&cspace, slot, &Permissions::write_only(), None, 0);

        assert!(matches!(result, Err(AxiomError::InsufficientRights)));
    }

    #[test]
    fn test_axiom_check_expired_capability() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 1000,
        };
        let slot = cspace.insert(cap);

        let result = axiom_check(&cspace, slot, &Permissions::read_only(), None, 2000);

        assert!(matches!(result, Err(AxiomError::Expired)));
    }

    #[test]
    fn test_capability_never_expires() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0, // 0 = never expires
        };
        let slot = cspace.insert(cap);

        // Even with a huge current_time, should not expire
        let result = axiom_check(&cspace, slot, &Permissions::read_only(), None, u64::MAX);

        assert!(result.is_ok());
    }

    #[test]
    fn test_permission_subset() {
        let full = Permissions::full();
        let read = Permissions::read_only();
        let write = Permissions::write_only();

        assert!(read.is_subset_of(&full));
        assert!(write.is_subset_of(&full));
        assert!(!full.is_subset_of(&read));
    }

    // ========================================================================
    // CapabilitySpace tests
    // ========================================================================

    #[test]
    fn test_capability_space_len_and_is_empty() {
        let mut cspace = CapabilitySpace::new();
        assert!(cspace.is_empty());
        assert_eq!(cspace.len(), 0);

        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        cspace.insert(cap);

        assert!(!cspace.is_empty());
        assert_eq!(cspace.len(), 1);
    }

    #[test]
    fn test_capability_space_contains() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        assert!(cspace.contains(slot));
        assert!(!cspace.contains(999));
    }

    #[test]
    fn test_capability_space_remove() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        assert!(cspace.contains(slot));
        let removed = cspace.remove(slot);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, 1);
        assert!(!cspace.contains(slot));

        // Remove non-existent slot returns None
        assert!(cspace.remove(999).is_none());
    }

    #[test]
    fn test_capability_space_list() {
        let mut cspace = CapabilitySpace::new();
        let cap1 = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let cap2 = Capability {
            id: 2,
            object_type: ObjectType::Process,
            object_id: 100,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };

        let slot1 = cspace.insert(cap1);
        let slot2 = cspace.insert(cap2);

        let list = cspace.list();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|(s, c)| *s == slot1 && c.id == 1));
        assert!(list.iter().any(|(s, c)| *s == slot2 && c.id == 2));
    }

    #[test]
    fn test_capability_space_get_mut() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        // Modify capability via get_mut
        if let Some(cap_mut) = cspace.get_mut(slot) {
            cap_mut.permissions.write = true;
        }

        let updated = cspace.get(slot).unwrap();
        assert!(updated.permissions.write);
    }

    #[test]
    fn test_capability_space_next_slot_increments() {
        let mut cspace = CapabilitySpace::new();
        assert_eq!(cspace.next_slot, 0);

        let cap1 = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot1 = cspace.insert(cap1);
        assert_eq!(slot1, 0);
        assert_eq!(cspace.next_slot, 1);

        let cap2 = Capability {
            id: 2,
            object_type: ObjectType::Endpoint,
            object_id: 43,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot2 = cspace.insert(cap2);
        assert_eq!(slot2, 1);
        assert_eq!(cspace.next_slot, 2);
    }

    // ========================================================================
    // has_permissions tests - all combinations
    // ========================================================================

    #[test]
    fn test_has_permissions_all_combinations() {
        // Full permissions capability
        let full_cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };

        // Full cap should satisfy any permission combination
        assert!(full_cap.has_permissions(&Permissions::full()));
        assert!(full_cap.has_permissions(&Permissions::read_only()));
        assert!(full_cap.has_permissions(&Permissions::write_only()));
        assert!(full_cap.has_permissions(&Permissions { read: false, write: false, grant: true }));
        assert!(full_cap.has_permissions(&Permissions { read: true, write: true, grant: false }));
        assert!(full_cap.has_permissions(&Permissions::default())); // No permissions required

        // Read-only capability
        let read_cap = Capability {
            id: 2,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };

        assert!(read_cap.has_permissions(&Permissions::read_only()));
        assert!(read_cap.has_permissions(&Permissions::default()));
        assert!(!read_cap.has_permissions(&Permissions::write_only()));
        assert!(!read_cap.has_permissions(&Permissions::full()));
        assert!(!read_cap.has_permissions(&Permissions { read: false, write: false, grant: true }));

        // Write-only capability
        let write_cap = Capability {
            id: 3,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::write_only(),
            generation: 0,
            expires_at: 0,
        };

        assert!(write_cap.has_permissions(&Permissions::write_only()));
        assert!(write_cap.has_permissions(&Permissions::default()));
        assert!(!write_cap.has_permissions(&Permissions::read_only()));
        assert!(!write_cap.has_permissions(&Permissions::full()));

        // Grant-only capability
        let grant_cap = Capability {
            id: 4,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions { read: false, write: false, grant: true },
            generation: 0,
            expires_at: 0,
        };

        assert!(grant_cap.has_permissions(&Permissions { read: false, write: false, grant: true }));
        assert!(grant_cap.has_permissions(&Permissions::default()));
        assert!(!grant_cap.has_permissions(&Permissions::read_only()));
        assert!(!grant_cap.has_permissions(&Permissions::write_only()));
    }

    // ========================================================================
    // axiom_check edge cases
    // ========================================================================

    #[test]
    fn test_axiom_check_type_agnostic() {
        // When expected_type is None, any object type should pass
        let mut cspace = CapabilitySpace::new();

        let endpoint_cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot1 = cspace.insert(endpoint_cap);

        let process_cap = Capability {
            id: 2,
            object_type: ObjectType::Process,
            object_id: 100,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot2 = cspace.insert(process_cap);

        // Both should pass with expected_type: None
        let result1 = axiom_check(&cspace, slot1, &Permissions::read_only(), None, 0);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap().object_type, ObjectType::Endpoint);

        let result2 = axiom_check(&cspace, slot2, &Permissions::read_only(), None, 0);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap().object_type, ObjectType::Process);
    }

    #[test]
    fn test_axiom_check_exactly_at_expiry() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 1000,
        };
        let slot = cspace.insert(cap);

        // At exactly the expiry time (current_time == expires_at), should NOT be expired
        // Because is_expired checks current_time > expires_at (not >=)
        let result_at_expiry = axiom_check(&cspace, slot, &Permissions::read_only(), None, 1000);
        assert!(result_at_expiry.is_ok(), "At exactly expiry time should still be valid");

        // One tick after expiry
        let result_after_expiry = axiom_check(&cspace, slot, &Permissions::read_only(), None, 1001);
        assert!(matches!(result_after_expiry, Err(AxiomError::Expired)));
    }

    #[test]
    fn test_axiom_check_zero_permissions_required() {
        let mut cspace = CapabilitySpace::new();
        let cap = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::default(), // No permissions
            generation: 0,
            expires_at: 0,
        };
        let slot = cspace.insert(cap);

        // Should pass when requiring no permissions
        let result = axiom_check(&cspace, slot, &Permissions::default(), None, 0);
        assert!(result.is_ok());

        // Should fail when requiring any permission
        let result = axiom_check(&cspace, slot, &Permissions::read_only(), None, 0);
        assert!(matches!(result, Err(AxiomError::InsufficientRights)));
    }

    #[test]
    fn test_axiom_check_all_object_types() {
        let mut cspace = CapabilitySpace::new();

        let object_types = [
            ObjectType::Endpoint,
            ObjectType::Process,
            ObjectType::Memory,
            ObjectType::Irq,
            ObjectType::IoPort,
            ObjectType::Console,
        ];

        for (i, obj_type) in object_types.iter().enumerate() {
            let cap = Capability {
                id: i as u64,
                object_type: *obj_type,
                object_id: 42,
                permissions: Permissions::full(),
                generation: 0,
                expires_at: 0,
            };
            let slot = cspace.insert(cap);

            // Check with matching type
            let result = axiom_check(&cspace, slot, &Permissions::read_only(), Some(*obj_type), 0);
            assert!(result.is_ok(), "Should pass for {:?}", obj_type);

            // Check with wrong type
            let wrong_type = if *obj_type == ObjectType::Endpoint {
                ObjectType::Process
            } else {
                ObjectType::Endpoint
            };
            let result = axiom_check(&cspace, slot, &Permissions::read_only(), Some(wrong_type), 0);
            assert!(matches!(result, Err(AxiomError::WrongType)), "Should fail with wrong type for {:?}", obj_type);
        }
    }

    #[test]
    fn test_capability_is_expired() {
        // Never expires (expires_at = 0)
        let cap_never = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        assert!(!cap_never.is_expired(0));
        assert!(!cap_never.is_expired(u64::MAX));

        // Expires at 1000
        let cap_expires = Capability {
            id: 2,
            object_type: ObjectType::Endpoint,
            object_id: 42,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 1000,
        };
        assert!(!cap_expires.is_expired(0));
        assert!(!cap_expires.is_expired(999));
        assert!(!cap_expires.is_expired(1000)); // At exactly expiry time
        assert!(cap_expires.is_expired(1001));
        assert!(cap_expires.is_expired(u64::MAX));
    }
}
