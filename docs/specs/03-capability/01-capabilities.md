# Capability Service Specification

**Version:** 1.0  
**Status:** Specification  
**Component:** Layer 3 - Process & Capability

---

## 1. Overview

The Capability Service manages **unforgeable access tokens** that grant specific permissions to resources. Capabilities can be delegated with attenuation (permission reduction) and revoked with cascade to descendants.

### 1.1 Position in Architecture

| Layer | Component | Relationship |
|-------|-----------|--------------|
| Layer 2 | Policy Engine | Authorizes capability operations |
| Layer 2 | Axiom | Records all capability grants and revocations |
| Layer 2 | Identity Service | Identifies holders and grantors |
| **Layer 3** | **Capability Service** | **Manages capability delegation** |
| Layer 4+ | All Services | Request and use capabilities |

### 1.2 Design Principles

| Principle | Description |
|-----------|-------------|
| **Unforgeable** | Capabilities cannot be fabricated — only granted by authorized entities |
| **Attenuatable** | Delegated capabilities can only reduce permissions, never amplify |
| **Revocable** | Any grantor can revoke, with cascade to all descendants |
| **Auditable** | All operations recorded in Axiom |

---

## 2. Capability Structure

```rust
/// A capability token (unforgeable reference)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    /// Unique identifier
    pub id: CapabilityId,
    
    /// Object type this grants access to
    pub object_type: ObjectType,
    
    /// Object identifier
    pub object_id: ResourceId,
    
    /// Granted permissions
    pub permissions: Permissions,
    
    /// Optional restrictions
    pub restrictions: Option<Restrictions>,
    
    /// Who holds this capability
    pub holder: IdentityId,
    
    /// Who granted this capability
    pub grantor: IdentityId,
    
    /// Parent capability (for delegation chain)
    pub parent: Option<CapabilityId>,
    
    /// Generation (for revocation)
    pub generation: u64,
    
    /// Axiom entry that created this capability
    pub created_at: AxiomRef,
}

/// Permission bits
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Permissions {
    /// Can read/receive
    pub read: bool,
    
    /// Can write/send
    pub write: bool,
    
    /// Can execute
    pub execute: bool,
    
    /// Can delegate to others
    pub delegate: bool,
    
    /// Can revoke delegated capabilities
    pub revoke: bool,
}

/// Object types that capabilities can reference
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    /// File or directory
    File,
    
    /// IPC endpoint
    Endpoint,
    
    /// Process
    Process,
    
    /// Memory region
    Memory,
    
    /// Network socket
    Socket,
    
    /// Device
    Device,
    
    /// Service
    Service,
}
```

---

## 3. Capability Service Interface

```rust
/// Capability Service interface
pub trait CapabilityService {
    /// Grant a capability to an identity
    fn grant(
        &mut self,
        request: CapabilityGrantRequest,
        auth: AuthToken,
    ) -> Result<CapabilityId, CapabilityError>;
    
    /// Delegate a capability (attenuated)
    fn delegate(
        &mut self,
        capability_id: CapabilityId,
        to: IdentityId,
        permissions: Permissions,  // Must be subset of original
        auth: AuthToken,
    ) -> Result<CapabilityId, CapabilityError>;
    
    /// Revoke a capability
    fn revoke(
        &mut self,
        capability_id: CapabilityId,
        auth: AuthToken,
    ) -> Result<(), CapabilityError>;
    
    /// Check if identity holds capability
    fn check(
        &self,
        identity: IdentityId,
        resource: ResourceId,
        permission: Permission,
    ) -> Result<bool, CapabilityError>;
    
    /// List capabilities held by identity
    fn list(
        &self,
        identity: IdentityId,
        filter: CapabilityFilter,
    ) -> Result<Vec<Capability>, CapabilityError>;
    
    /// Get capability chain (delegation history)
    fn get_chain(
        &self,
        capability_id: CapabilityId,
    ) -> Result<Vec<Capability>, CapabilityError>;
}
```

---

## 4. Capability Grant

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityGrantRequest {
    /// Resource to grant access to
    pub resource: ResourceId,
    
    /// Permissions to grant
    pub permissions: Permissions,
    
    /// Identity to grant to
    pub grantee: IdentityId,
    
    /// Optional restrictions
    pub restrictions: Option<Restrictions>,
    
    /// Expiration (optional)
    pub expires_at: Option<AxiomTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Restrictions {
    /// Time-based restrictions
    pub time: Option<TimeRestriction>,
    
    /// Rate limiting
    pub rate_limit: Option<RateLimit>,
    
    /// Context restrictions
    pub context: Option<ContextRestriction>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeRestriction {
    /// Valid after this time
    pub not_before: Option<AxiomTime>,
    
    /// Valid until this time
    pub not_after: Option<AxiomTime>,
    
    /// Valid during these hours (e.g., business hours)
    pub time_of_day: Option<TimeOfDayRestriction>,
}
```

---

## 5. Capability Delegation

Capabilities can be delegated with attenuation (reduction of permissions):

```rust
impl CapabilityService {
    fn delegate(
        &mut self,
        capability_id: CapabilityId,
        to: IdentityId,
        new_permissions: Permissions,
        auth: AuthToken,
    ) -> Result<CapabilityId, CapabilityError> {
        // 1. Get the source capability
        let source = self.get(capability_id)?;
        
        // 2. Verify caller holds the capability
        if source.holder != auth.identity {
            return Err(CapabilityError::NotHolder);
        }
        
        // 3. Check delegate permission
        if !source.permissions.delegate {
            return Err(CapabilityError::CannotDelegate);
        }
        
        // 4. Verify new permissions are subset
        if !new_permissions.is_subset_of(&source.permissions) {
            return Err(CapabilityError::CannotAmplify);
        }
        
        // 5. Request policy authorization
        let decision = self.policy_engine.evaluate(PolicyRequest {
            requestor: auth.identity,
            action: PolicyAction::DelegateCapability {
                capability: capability_id,
                to,
            },
            ..Default::default()
        })?;
        
        if !decision.is_allowed() {
            return Err(CapabilityError::PolicyDenied(decision));
        }
        
        // 6. Create new capability
        let new_cap = Capability {
            id: CapabilityId::new(),
            object_type: source.object_type,
            object_id: source.object_id,
            permissions: new_permissions,
            restrictions: source.restrictions.clone(),
            holder: to,
            grantor: auth.identity,
            parent: Some(capability_id),
            generation: source.generation,
            created_at: decision.axiom_ref.unwrap(),
        };
        
        // 7. Record in Axiom
        self.record_grant(&new_cap)?;
        
        Ok(new_cap.id)
    }
}
```

---

## 6. Capability Revocation

Revocation cascades to all delegated capabilities:

```rust
impl CapabilityService {
    fn revoke(
        &mut self,
        capability_id: CapabilityId,
        auth: AuthToken,
    ) -> Result<(), CapabilityError> {
        let cap = self.get(capability_id)?;
        
        // Verify caller can revoke (grantor or has revoke permission)
        if cap.grantor != auth.identity && !self.can_revoke(&auth, &cap)? {
            return Err(CapabilityError::CannotRevoke);
        }
        
        // Find all capabilities derived from this one
        let descendants = self.find_descendants(capability_id)?;
        
        // Request policy authorization
        let decision = self.policy_engine.evaluate(PolicyRequest {
            requestor: auth.identity,
            action: PolicyAction::RevokeCapability { capability: capability_id },
            ..Default::default()
        })?;
        
        if !decision.is_allowed() {
            return Err(CapabilityError::PolicyDenied(decision));
        }
        
        // Revoke all (recorded as single Axiom entry)
        let mut to_revoke = vec![capability_id];
        to_revoke.extend(descendants.iter().map(|c| c.id));
        
        self.record_revocation(&to_revoke, decision.axiom_ref.unwrap())?;
        
        Ok(())
    }
}
```

---

## 7. Capability Flow

```
┌──────────────────────────────────────────────────────────────┐
│                    CAPABILITY DELEGATION                      │
│                                                              │
│   ┌─────────┐         ┌─────────┐         ┌─────────┐       │
│   │  Root   │ ──────▶ │ Service │ ──────▶ │  User   │       │
│   │ (Super) │ grants  │(fs)     │ grants  │ (alice) │       │
│   └─────────┘         └─────────┘         └─────────┘       │
│                                                              │
│   [RWD]       ───▶    [RW]        ───▶    [R]               │
│   (full)             (reduced)           (read-only)         │
│                                                              │
│   Delegation chain recorded in Axiom                        │
│   Revocation of [RW] also revokes [R]                       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## 8. Axiom Integration

### 8.1 Capability Events

```rust
/// Axiom entry types for capability operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CapabilityAxiomEntry {
    /// Capability granted
    CapabilityGranted {
        capability: Capability,
        granted_by: IdentityId,
    },
    
    /// Capability delegated
    CapabilityDelegated {
        source: CapabilityId,
        new_capability: Capability,
        delegated_by: IdentityId,
    },
    
    /// Capability revoked (with cascade list)
    CapabilityRevoked {
        revoked: Vec<CapabilityId>,
        revoked_by: IdentityId,
        reason: String,
    },
}
```

---

## 9. Implementation Notes

### 9.1 Capability Storage

Capabilities are stored in two places:
1. **Axiom** — Authoritative record of all grants/revocations
2. **Kernel** — Per-process capability table for fast access checks

### 9.2 Performance Targets

| Operation | Target Latency |
|-----------|----------------|
| Capability check | < 1μs (kernel table lookup) |
| Capability grant | < 1ms (includes Axiom write) |
| Capability revocation | < 10ms (includes cascade) |

### 9.3 Security Properties

| Property | Guarantee |
|----------|-----------|
| **Unforgeable** | Capabilities cannot be fabricated |
| **Attenuatable** | Delegated capabilities can only reduce permissions |
| **Revocable** | Any grantor can revoke, cascade to descendants |
| **Auditable** | All operations recorded in Axiom |

---

*[← Identity Service](../02-authority/04-identity.md) | [Process Manager →](02-processes.md)*
