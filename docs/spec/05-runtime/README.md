# Runtime Services

> Runtime services run in user-space and provide OS functionality above the kernel.

## Overview

Unlike traditional OS designs where these are kernel subsystems, Orbital OS implements them as user-space services communicating via IPC. This provides:

1. **Isolation**: Service bugs don't crash the kernel
2. **Updatability**: Services can be updated without reboot
3. **Flexibility**: Different policy implementations possible

## Services

| Service                                   | Description                         |
|-------------------------------------------|-------------------------------------|
| [01-process.md](01-process.md)            | Process lifecycle management        |
| [02-permissions.md](02-permissions.md)    | Capability policy enforcement       |
| [03-identity.md](03-identity.md)          | User/service identity               |
| [04-storage.md](04-storage.md)            | Persistent storage access           |
| [05-network.md](05-network.md)            | Network connectivity                |

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Applications                                  │
│                                                                     │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐                       │
│  │ App 1  │ │ App 2  │ │ App 3  │ │ Shell  │                       │
│  └───┬────┘ └───┬────┘ └───┬────┘ └───┬────┘                       │
│      │          │          │          │                            │
└──────┼──────────┼──────────┼──────────┼────────────────────────────┘
       │          │          │          │
       │    IPC   │    IPC   │    IPC   │
       │          │          │          │
┌──────▼──────────▼──────────▼──────────▼────────────────────────────┐
│                       Runtime Services                               │
│                                                                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│  │ Process  │ │ Perms    │ │ Identity │ │ Storage  │ │ Network  │ │
│  │ Manager  │ │ Service  │ │ Service  │ │ Service  │ │ Service  │ │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ │
│       │            │            │            │            │        │
└───────┼────────────┼────────────┼────────────┼────────────┼────────┘
        │            │            │            │            │
        │      Syscalls           │            │            │
        │            │            │            │            │
┌───────▼────────────▼────────────▼────────────▼────────────▼────────┐
│                     Axiom (Verification Layer)                       │
│                                                                     │
│  SysLog (audit) │ CommitLog (state) │ Sender Verification           │
└───────┬────────────┬────────────┬────────────┬────────────┬────────┘
        │            │            │            │            │
        │      Forward to Kernel  │            │            │
        │            │            │            │            │
┌───────▼────────────▼────────────▼────────────▼────────────▼────────┐
│                           Kernel                                     │
│                                                                     │
│  Capabilities │ Threads │ VMM │ IPC │ Interrupts                    │
│                                                                     │
│  → Emits Commits for all state changes                              │
└─────────────────────────────────────────────────────────────────────┘
```

## Service Discovery

Services register their endpoints with init:

```rust
/// Service registration message.
pub const MSG_REGISTER_SERVICE: u32 = 0x3000;

/// Register a service with the service registry.
pub struct ServiceRegistration {
    /// Service type (e.g., "storage", "network")
    pub service_type: String,
    /// Endpoint capability slot
    pub endpoint_slot: CapSlot,
    /// Service version
    pub version: String,
}
```

Applications discover services via init:

```rust
/// Service lookup request.
pub const MSG_LOOKUP_SERVICE: u32 = 0x3001;

/// Service lookup response.
pub struct ServiceLookup {
    /// Requested service type
    pub service_type: String,
    /// Endpoint capability (if found)
    pub endpoint_cap: Option<Capability>,
}
```

## Common IPC Patterns

### Request-Response

```rust
// Client side
fn call_service(service_ep: CapSlot, request: &[u8]) -> Result<Vec<u8>, Error> {
    // Create reply endpoint
    let reply_ep = create_endpoint();
    
    // Send request with reply endpoint capability
    send_with_caps(service_ep, MSG_REQUEST, request, &[reply_ep])?;
    
    // Wait for response
    let response = receive_blocking(reply_ep);
    
    // Cleanup
    delete_endpoint(reply_ep);
    
    Ok(response.data)
}

// Service side
fn handle_request(msg: ReceivedMessage) {
    let reply_ep = msg.cap_slots[0];  // Client's reply endpoint
    
    // Process request
    let result = process(&msg.data);
    
    // Send response
    send(reply_ep, MSG_RESPONSE, &result);
}
```

### Notification

```rust
// Subscriber registers
fn subscribe(service_ep: CapSlot, event_type: u32) {
    let notification_ep = create_endpoint();
    send_with_caps(service_ep, MSG_SUBSCRIBE, &event_type.to_le_bytes(), &[notification_ep]);
}

// Publisher notifies
fn notify_subscribers(event: &Event) {
    for subscriber in &self.subscribers {
        send(subscriber.endpoint, MSG_NOTIFICATION, &event.encode());
    }
}
```

## Capability Flow

Capabilities flow from init through runtime services to applications:

```
     init                 ProcessManager              Application
       │                        │                          │
       │ Grant ProcessCap       │                          │
       │───────────────────────▶│                          │
       │                        │                          │
       │                        │ Grant AttenuatedCap      │
       │                        │─────────────────────────▶│
       │                        │                          │
       │                        │                          │
       │                        │                          │ Use cap via syscall
       │                        │                          │──────────────────▶
       │                        │                          │
```

## WASM Implementation

On WASM, runtime services are regular WASM modules:

```rust
// storage_service.rs
#![no_std]
extern crate orbital_process;

use orbital_process::*;

#[no_mangle]
pub extern "C" fn _start() {
    debug("storage: starting");
    
    // Create our service endpoint
    let service_ep = create_endpoint();
    
    // Register with init
    register_service("storage", service_ep);
    
    // Signal ready
    send_ready();
    
    // Main service loop
    loop {
        let msg = receive_blocking(service_ep);
        match msg.tag {
            tags::STORAGE_READ => handle_read(msg),
            tags::STORAGE_WRITE => handle_write(msg),
            tags::STORAGE_DELETE => handle_delete(msg),
            _ => send_error(msg.reply_ep, ERROR_UNKNOWN_REQUEST),
        }
    }
}
```

## Service Capabilities

Each service receives specific capabilities from init:

| Service        | Capabilities Received                          |
|----------------|------------------------------------------------|
| ProcessManager | Process creation, capability manipulation       |
| Permissions    | Read Axiom log, policy storage                 |
| Identity       | Secure storage for keys                        |
| Storage        | IndexedDB/disk access                          |
| Network        | Fetch API/socket access                        |
