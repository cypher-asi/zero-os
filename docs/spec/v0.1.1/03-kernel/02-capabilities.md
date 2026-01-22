# Capabilities

## Overview

Zero OS uses capability-based security. All resource access requires holding an appropriate capability in the process's CSpace.

## Capability Structure

```rust
pub struct Capability {
    /// Unique capability ID
    pub id: u64,
    /// Type of object this capability references
    pub object_type: ObjectType,
    /// ID of the referenced object
    pub object_id: u64,
    /// Permission bits
    pub permissions: Permissions,
}

#[repr(u8)]
pub enum ObjectType {
    Endpoint = 1,  // IPC endpoint
    Console = 2,   // Console I/O
    Storage = 3,   // Persistent storage
    Network = 4,   // Network access
    Process = 5,   // Process management
    Memory = 6,    // Memory region
}

pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub grant: bool,
}
```

## CSpace (Capability Space)

Each process has a CSpace—a table of capability slots:

```rust
pub struct CSpace {
    caps: BTreeMap<CapSlot, Capability>,
    next_slot: CapSlot,
}

pub type CapSlot = u32;
```

### Well-Known Slots

| Slot | Purpose |
|------|---------|
| 0 | Process's own endpoint (for receiving) |
| 1 | Input endpoint (for terminal) |
| 2 | Init endpoint (for service discovery) |

## Capability Operations

### Grant

Transfer a capability to another process (with optional permission reduction):

```rust
// Syscall: SYS_CAP_GRANT (0x30)
// Args: from_slot, to_pid, perms

fn cap_grant(&mut self, from_pid: ProcessId, from_slot: CapSlot, to_pid: ProcessId, perms: Permissions) 
    -> Result<CapSlot, KernelError> 
{
    // 1. Verify from_pid has capability
    let cap = self.cspaces.get(&from_pid)?.get(from_slot)?;
    
    // 2. Verify from_pid has grant permission
    if !cap.permissions.grant {
        return Err(KernelError::PermissionDenied);
    }
    
    // 3. Create new capability with attenuated permissions
    let new_perms = cap.permissions.intersect(perms);
    let new_cap = Capability {
        id: self.next_cap_id(),
        object_type: cap.object_type,
        object_id: cap.object_id,
        permissions: new_perms,
    };
    
    // 4. Insert into target's CSpace
    let to_slot = self.cspaces.get_mut(&to_pid)?.insert(new_cap);
    
    // 5. Record in Axiom
    self.axiom.commit(CommitType::CapGranted { ... });
    
    Ok(to_slot)
}
```

### Revoke

Remove a capability from a process:

```rust
// Syscall: SYS_CAP_REVOKE (0x31)
// Args: slot

fn cap_revoke(&mut self, pid: ProcessId, slot: CapSlot) -> Result<(), KernelError> {
    // Verify process owns the capability
    let cap = self.cspaces.get(&pid)?.get(slot)?;
    
    // Remove capability
    self.cspaces.get_mut(&pid)?.remove(slot);
    
    // Record in Axiom
    self.axiom.commit(CommitType::CapRemoved { pid: pid.0, slot });
    
    Ok(())
}
```

### Delete

Remove a capability from own CSpace (no notification):

```rust
// Syscall: SYS_CAP_DELETE (0x32)
// Args: slot

fn cap_delete(&mut self, pid: ProcessId, slot: CapSlot) -> Result<(), KernelError> {
    self.cspaces.get_mut(&pid)?.remove(slot)?;
    self.axiom.commit(CommitType::CapRemoved { pid: pid.0, slot });
    Ok(())
}
```

### Inspect

Query information about a capability:

```rust
// Syscall: SYS_CAP_INSPECT (0x33)
// Args: slot
// Returns: CapInfo in data buffer

fn cap_inspect(&self, pid: ProcessId, slot: CapSlot) -> Result<CapInfo, KernelError> {
    let cap = self.cspaces.get(&pid)?.get(slot)?;
    Ok(CapInfo {
        slot,
        object_type: cap.object_type as u8,
        object_id: cap.object_id,
        can_read: cap.permissions.read,
        can_write: cap.permissions.write,
        can_grant: cap.permissions.grant,
    })
}
```

### Derive

Create a new capability with reduced permissions:

```rust
// Syscall: SYS_CAP_DERIVE (0x34)
// Args: slot, new_perms

fn cap_derive(&mut self, pid: ProcessId, slot: CapSlot, new_perms: Permissions) 
    -> Result<CapSlot, KernelError> 
{
    let cap = self.cspaces.get(&pid)?.get(slot)?;
    
    // Attenuate permissions
    let derived_perms = cap.permissions.intersect(new_perms);
    
    let new_cap = Capability {
        id: self.next_cap_id(),
        object_type: cap.object_type,
        object_id: cap.object_id,
        permissions: derived_perms,
    };
    
    let new_slot = self.cspaces.get_mut(&pid)?.insert(new_cap);
    self.axiom.commit(CommitType::CapInserted { ... });
    
    Ok(new_slot)
}
```

## Revocation Notification

When a capability is revoked by an external party (supervisor, permission manager), the affected process receives a notification:

```rust
pub struct RevocationNotification {
    pub pid: ProcessId,
    pub slot: CapSlot,
    pub object_type: u8,
    pub object_id: u64,
    pub reason: RevocationReason,
}

#[repr(u8)]
pub enum RevocationReason {
    Explicit = 1,      // Supervisor/user revoked
    Expired = 2,       // Capability expired
    ProcessExit = 3,   // Source process exited
}
```

The notification is delivered as an IPC message to the process's input endpoint:

```
MSG_CAP_REVOKED (0x3010)
├── slot: u32
├── object_type: u8
├── object_id: u64
└── reason: u8
```

## Permission Checking

All kernel operations check capabilities before proceeding:

```rust
fn check_permission(&self, pid: ProcessId, slot: CapSlot, required: Permissions) 
    -> Result<&Capability, KernelError> 
{
    let cap = self.cspaces.get(&pid.0)
        .ok_or(KernelError::ProcessNotFound)?
        .get(slot)
        .ok_or(KernelError::InvalidCapability)?;
    
    if required.read && !cap.permissions.read {
        return Err(KernelError::PermissionDenied);
    }
    if required.write && !cap.permissions.write {
        return Err(KernelError::PermissionDenied);
    }
    if required.grant && !cap.permissions.grant {
        return Err(KernelError::PermissionDenied);
    }
    
    Ok(cap)
}
```

## Compliance Checklist

### Source Files
- `crates/zos-kernel/src/lib.rs` - Capability management
- `crates/zos-process/src/lib.rs` - ObjectType, Permissions

### Key Invariants
- [ ] Capabilities are unforgeable (kernel-controlled IDs)
- [ ] Permissions can only be reduced, never expanded
- [ ] Grant requires grant permission
- [ ] Revocation notifies affected process
- [ ] All cap mutations recorded in CommitLog

### Differences from v0.1.0
- Added RevocationNotification delivery
- Added derive syscall for in-process attenuation
- ObjectType is repr(u8) for wire format
- Privileged kernel API for supervisor revocation
