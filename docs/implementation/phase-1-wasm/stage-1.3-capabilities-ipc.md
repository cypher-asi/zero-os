# Stage 1.3: Capabilities + IPC

> **Status**: ✅ **COMPLETE**
>
> **Goal**: Implement capability system and IPC message passing.

## Implementation Status

This stage is **fully implemented** with comprehensive functionality.

### What's Implemented

| Component | Status | Location |
|-----------|--------|----------|
| Capability struct | ✅ | `crates/orbital-kernel/src/lib.rs:454-474` |
| CapabilitySpace | ✅ | `crates/orbital-kernel/src/lib.rs:477-529` |
| Permissions (read/write/grant) | ✅ | `crates/orbital-kernel/src/lib.rs:413-450` |
| ObjectType enum | ✅ | `crates/orbital-kernel/src/lib.rs:108-123` |
| `axiom_check()` function | ✅ | `crates/orbital-kernel/src/lib.rs:379-410` |
| Endpoint struct | ✅ | `crates/orbital-kernel/src/lib.rs:562-572` |
| Message struct | ✅ | `crates/orbital-kernel/src/lib.rs:550-560` |
| IPC with cap transfer | ✅ | `crates/orbital-kernel/src/lib.rs:1218-1350` |
| SYS_EP_CREATE | ✅ | `crates/orbital-kernel/src/lib.rs:903-957` |
| SYS_SEND | ✅ | `crates/orbital-kernel/src/lib.rs:1128-1201` |
| SYS_RECV | ✅ | `crates/orbital-kernel/src/lib.rs:1395-1437` |
| SYS_CAP_GRANT | ✅ | `crates/orbital-kernel/src/lib.rs:959-1021` |
| SYS_CAP_REVOKE | ✅ | `crates/orbital-kernel/src/lib.rs:1036-1075` |
| SYS_CAP_DELETE | ✅ | `crates/orbital-kernel/src/lib.rs:1091-1125` |
| SYS_CAP_DERIVE | ✅ | `crates/orbital-kernel/src/lib.rs:1601-1656` |
| SYS_CAP_INSPECT | ✅ | Via `Syscall::CapInspect` |
| SYS_SEND_CAP | ✅ | `crates/orbital-kernel/src/lib.rs:1218-1350` |
| Process syscall library | ✅ | `crates/orbital-process/src/lib.rs:381-638` |

### Key Implementation Details

#### Capability Structure

```rust
// crates/orbital-kernel/src/lib.rs
pub struct Capability {
    pub id: u64,
    pub object_type: ObjectType,
    pub object_id: u64,
    pub permissions: Permissions,
    pub generation: u32,
    pub expires_at: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub grant: bool,
}

pub enum ObjectType {
    Endpoint = 1,
    Process = 2,
    Memory = 3,
    Irq = 4,
    IoPort = 5,
    Console = 6,
}
```

#### Capability Checking (Axiom)

```rust
// crates/orbital-kernel/src/lib.rs
pub fn axiom_check<'a>(
    cspace: &'a CapabilitySpace,
    slot: CapSlot,
    required: &Permissions,
    expected_type: Option<ObjectType>,
    current_time: u64,
) -> Result<&'a Capability, AxiomError> {
    // 1. Lookup capability
    let cap = cspace.get(slot).ok_or(AxiomError::InvalidSlot)?;
    
    // 2. Check object type
    if let Some(expected) = expected_type {
        if cap.object_type != expected { return Err(AxiomError::WrongType); }
    }
    
    // 3. Check permissions
    if (required.read && !cap.permissions.read)
        || (required.write && !cap.permissions.write)
        || (required.grant && !cap.permissions.grant)
    {
        return Err(AxiomError::InsufficientRights);
    }
    
    // 4. Check expiration
    if cap.is_expired(current_time) { return Err(AxiomError::Expired); }
    
    Ok(cap)
}
```

#### IPC with Capability Transfer

```rust
// crates/orbital-kernel/src/lib.rs
pub fn ipc_send_with_caps(
    &mut self,
    from_pid: ProcessId,
    endpoint_slot: CapSlot,
    tag: u32,
    data: Vec<u8>,
    cap_slots: &[CapSlot],
) -> Result<(), KernelError> {
    // 1. Validate endpoint capability
    // 2. Validate capabilities to transfer exist
    // 3. Remove capabilities from sender's CSpace
    // 4. Log transfers to Axiom
    // 5. Queue message with transferred capabilities
    // ...
}

pub fn ipc_receive_with_caps(
    &mut self,
    pid: ProcessId,
    endpoint_slot: CapSlot,
) -> Result<Option<(Message, Vec<CapSlot>)>, KernelError> {
    // 1. Receive message
    // 2. Install transferred capabilities into receiver's CSpace
    // 3. Return message and installed slot numbers
    // ...
}
```

### Syscall Numbers

```rust
// Capability syscalls (0x30 - 0x3F)
pub const SYS_CAP_GRANT: u32 = 0x30;
pub const SYS_CAP_REVOKE: u32 = 0x31;
pub const SYS_CAP_DELETE: u32 = 0x32;
pub const SYS_CAP_INSPECT: u32 = 0x33;
pub const SYS_CAP_DERIVE: u32 = 0x34;
pub const SYS_CAP_LIST: u32 = 0x35;

// IPC syscalls (0x40 - 0x4F)
pub const SYS_SEND: u32 = 0x40;
pub const SYS_RECV: u32 = 0x41;
pub const SYS_CALL: u32 = 0x42;
pub const SYS_REPLY: u32 = 0x43;
pub const SYS_SEND_CAP: u32 = 0x44;

// Endpoint syscalls (0x11 - 0x12)
pub const SYS_CREATE_ENDPOINT: u32 = 0x11;
pub const SYS_DELETE_ENDPOINT: u32 = 0x12;
```

## Tests

All 40+ tests pass. Key capability and IPC tests:

```bash
cargo test -p orbital-kernel
```

### Test Coverage

| Test | Description |
|------|-------------|
| `test_endpoint_creation` | Create endpoint, get capability |
| `test_capability_grant` | Grant cap with attenuation |
| `test_capability_grant_requires_grant_permission` | No grant without grant perm |
| `test_ipc_send_receive` | Basic send/receive |
| `test_ipc_requires_capability` | IPC fails without cap |
| `test_ipc_requires_write_permission` | Send needs write perm |
| `test_ipc_metrics` | Message counting |
| `test_capability_revoke` | Revoke removes cap |
| `test_capability_delete` | Delete without grant perm |
| `test_capability_ipc_with_transfer` | Send caps via IPC |
| `test_capability_derive` | Derive with reduced perms |
| `test_axiom_check_*` | All capability checking |
| `test_capability_grant_chain` | Multi-hop grants |
| `test_axiom_log_full_workflow` | Create→Grant→Delete→Revoke |

## Invariants Verified

### 1. Capability Integrity ✅

- ✅ Capabilities only created by kernel (`create_endpoint`, `grant_capability`)
- ✅ Derived capabilities have permissions ≤ source (attenuated)
- ✅ Capability checks before every operation (`axiom_check`)
- ✅ No process can forge a capability (IDs assigned by kernel)

### 2. IPC Isolation ✅

- ✅ Messages queued at endpoint
- ✅ Only holders of endpoint capability can send/receive
- ✅ Write permission required for send
- ✅ Read permission required for receive
- ✅ Messages are FIFO ordered

### 3. Axiom Logging ✅

- ✅ CapOperation logged for Create, Grant, Revoke, Transfer, Delete
- ✅ Hash chain integrity maintained
- ✅ Timestamps recorded

## No Modifications Needed

This stage is complete. The implementation exceeds the original spec with:

- Capability expiration support
- Generation numbers for revocation tracking
- IPC metrics and traffic logging
- Enhanced syscall wrappers in `orbital-process`

## Process Library Usage

Processes can use the syscall library:

```rust
use orbital_process::{send, receive, cap_grant, cap_derive, Permissions};

// Send message
send(endpoint_slot, 0x1234, &data)?;

// Receive message
if let Some(msg) = receive(endpoint_slot) {
    // Process msg.from_pid, msg.tag, msg.data
}

// Grant capability
let new_slot = cap_grant(my_slot, target_pid, Permissions::read_only())?;

// Derive reduced capability
let derived = cap_derive(my_slot, Permissions { read: true, write: false, grant: false })?;
```

## Next Stage

Proceed to [Stage 1.4: Process Management](stage-1.4-process-management.md).
