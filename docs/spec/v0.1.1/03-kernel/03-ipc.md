# Inter-Process Communication (IPC)

## Overview

Zero OS IPC uses endpoints with message queues. Processes communicate by sending messages to endpoints they hold capabilities for.

## Endpoints

```rust
pub struct Endpoint {
    /// Unique endpoint ID
    pub id: EndpointId,
    /// Process that owns this endpoint
    pub owner: ProcessId,
    /// Message queue (FIFO)
    pub queue: VecDeque<Message>,
}

pub type EndpointId = u64;
```

## Messages

```rust
pub struct Message {
    /// Sender's process ID
    pub from_pid: ProcessId,
    /// Application-defined message tag
    pub tag: u32,
    /// Message payload
    pub data: Vec<u8>,
}
```

## Creating Endpoints

```rust
// Syscall: SYS_EP_CREATE (0x35)
// Returns: endpoint_id (high 32 bits in second call), slot (low 32 bits)

fn create_endpoint(&mut self, pid: ProcessId) -> Result<(EndpointId, CapSlot), KernelError> {
    // Allocate endpoint ID
    let endpoint_id = self.next_endpoint_id;
    self.next_endpoint_id += 1;
    
    // Create endpoint
    let endpoint = Endpoint {
        id: endpoint_id,
        owner: pid,
        queue: VecDeque::new(),
    };
    self.endpoints.insert(endpoint_id, endpoint);
    
    // Create capability for owner
    let cap = Capability {
        id: self.next_cap_id(),
        object_type: ObjectType::Endpoint,
        object_id: endpoint_id,
        permissions: Permissions::full(),
    };
    let slot = self.cspaces.get_mut(&pid.0)?.insert(cap);
    
    // Record in Axiom
    self.axiom.commit(CommitType::EndpointCreated { id: endpoint_id, owner: pid.0 });
    
    Ok((endpoint_id, slot))
}
```

## Sending Messages

```rust
// Syscall: SYS_SEND (0x40)
// Args: endpoint_slot, tag, data_len
// Data: message bytes (via syscall data buffer)

fn send(&mut self, pid: ProcessId, endpoint_slot: CapSlot, tag: u32, data: &[u8]) 
    -> Result<(), KernelError> 
{
    // Check capability
    let cap = self.check_permission(pid, endpoint_slot, Permissions::write_only())?;
    
    if cap.object_type != ObjectType::Endpoint {
        return Err(KernelError::InvalidCapability);
    }
    
    // Get endpoint
    let endpoint = self.endpoints.get_mut(&cap.object_id)
        .ok_or(KernelError::EndpointNotFound)?;
    
    // Queue message
    let message = Message {
        from_pid: pid,
        tag,
        data: data.to_vec(),
    };
    endpoint.queue.push_back(message);
    
    // Record in Axiom (metadata only)
    self.axiom.commit(CommitType::MessageSent {
        from_pid: pid.0,
        to_endpoint: cap.object_id,
        tag,
        size: data.len(),
    });
    
    // Wake any waiting receiver
    self.wake_receiver(endpoint.owner);
    
    Ok(())
}
```

## Receiving Messages

```rust
// Syscall: SYS_RECEIVE (0x41)
// Args: endpoint_slot
// Returns: message_len (0 if no message)
// Data: [from_pid: u32, tag: u32, data: bytes]

fn receive(&mut self, pid: ProcessId, endpoint_slot: CapSlot) 
    -> Result<Option<Message>, KernelError> 
{
    // Check capability
    let cap = self.check_permission(pid, endpoint_slot, Permissions::read_only())?;
    
    if cap.object_type != ObjectType::Endpoint {
        return Err(KernelError::InvalidCapability);
    }
    
    // Get endpoint
    let endpoint = self.endpoints.get_mut(&cap.object_id)
        .ok_or(KernelError::EndpointNotFound)?;
    
    // Verify ownership (only owner can receive)
    if endpoint.owner != pid {
        return Err(KernelError::PermissionDenied);
    }
    
    // Dequeue message (non-blocking)
    Ok(endpoint.queue.pop_front())
}
```

## RPC Pattern (Call/Reply)

### Call

Send a message and wait for reply:

```rust
// Syscall: SYS_CALL (0x42)
// Args: endpoint_slot, tag, data_len
// Blocks until reply received

fn call(&mut self, pid: ProcessId, endpoint_slot: CapSlot, tag: u32, data: &[u8]) 
    -> Result<Message, KernelError> 
{
    // Send the request
    self.send(pid, endpoint_slot, tag, data)?;
    
    // Wait for reply (in practice, handled by process yielding and polling)
    // ...
}
```

### Reply

Reply to a call:

```rust
// Syscall: SYS_REPLY (0x43)
// Args: caller_pid, tag, data_len

fn reply(&mut self, pid: ProcessId, caller_pid: u32, tag: u32, data: &[u8]) 
    -> Result<(), KernelError> 
{
    // Find caller's endpoint and send reply
    // (Implementation sends to caller's slot 0)
}
```

## Sending with Capabilities

Transfer capabilities along with a message:

```rust
// Syscall: SYS_SEND_CAP (0x44)
// Args: endpoint_slot, tag, data_len | (cap_count << 16)
// Data: [message_data, cap_slots...]

fn send_with_caps(
    &mut self, 
    pid: ProcessId, 
    endpoint_slot: CapSlot, 
    tag: u32, 
    data: &[u8],
    cap_slots: &[CapSlot],
) -> Result<(), KernelError> {
    // Send message
    self.send(pid, endpoint_slot, tag, data)?;
    
    // Transfer each capability
    for slot in cap_slots {
        // Move capability from sender to receiver
        // (Capability is removed from sender's CSpace)
    }
    
    Ok(())
}
```

## Console Input Delivery

The kernel provides a privileged API for the supervisor to deliver console input:

```rust
/// Deliver console input to a process (supervisor privilege)
pub fn deliver_console_input(
    &mut self,
    pid: ProcessId,
    endpoint_slot: CapSlot,
    data: &[u8],
) -> Result<(), KernelError> {
    // Get endpoint from process's CSpace
    let cap = self.cspaces.get(&pid.0)?.get(endpoint_slot)?;
    
    // Queue message with special tag
    let message = Message {
        from_pid: ProcessId(0), // From supervisor
        tag: MSG_CONSOLE_INPUT,
        data: data.to_vec(),
    };
    
    self.endpoints.get_mut(&cap.object_id)?.queue.push_back(message);
    Ok(())
}
```

## Message Tags

Well-known message tags:

| Tag | Name | Usage |
|-----|------|-------|
| 0x0002 | MSG_CONSOLE_INPUT | Terminal keyboard input |
| 0x1000 | MSG_REGISTER_SERVICE | Service registration |
| 0x1001 | MSG_LOOKUP_SERVICE | Service lookup |
| 0x1002 | MSG_LOOKUP_RESPONSE | Lookup result |
| 0x1003 | MSG_SPAWN_SERVICE | Request spawn |
| 0x1010 | MSG_GRANT_PERMISSION | Grant capability request |
| 0x1011 | MSG_REVOKE_PERMISSION | Revoke capability request |
| 0x2000 | MSG_APP_STATE | App state update |
| 0x2001 | MSG_APP_INPUT | App user input |
| 0x3010 | MSG_CAP_REVOKED | Capability revocation notification |

## Compliance Checklist

### Source Files
- `crates/zos-kernel/src/lib.rs` - IPC implementation
- `crates/zos-process/src/lib.rs` - Message tags

### Key Invariants
- [ ] Only endpoint owner can receive
- [ ] Send requires write permission
- [ ] Receive requires read permission
- [ ] Messages are FIFO ordered
- [ ] Capability transfer is atomic

### Differences from v0.1.0
- Added privileged console input delivery
- MessageSent commit records metadata only
- Cap transfer via SYS_SEND_CAP
- Non-blocking receive (no process state change)
