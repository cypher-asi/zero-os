# Application Model

> Applications run in sandboxed environments with capability-controlled access.

## Overview

Applications in Orbital OS:

1. **Sandboxed**: Run with minimal capabilities by default
2. **Capability-Request**: Must request capabilities they need
3. **Isolated**: Cannot access other apps' data without permission

## Application Lifecycle

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Application Lifecycle                           │
│                                                                     │
│  1. Install (future)                                                 │
│     └─ Store binary in storage                                       │
│     └─ Register with app manager                                     │
│                                                                     │
│  2. Launch                                                           │
│     └─ User/system requests launch                                   │
│     └─ ProcessManager spawns with manifest capabilities              │
│                                                                     │
│  3. Initialize                                                       │
│     └─ App receives initial capabilities                             │
│     └─ App can request additional capabilities                       │
│                                                                     │
│  4. Run                                                              │
│     └─ App uses granted capabilities                                 │
│     └─ IPC with services (storage, network, etc.)                    │
│                                                                     │
│  5. Terminate                                                        │
│     └─ App exits or is killed                                        │
│     └─ Capabilities revoked, resources freed                         │
└─────────────────────────────────────────────────────────────────────┘
```

## Application Manifest

Applications declare their requirements:

```rust
/// Application manifest.
#[derive(Clone, Debug)]
pub struct AppManifest {
    /// Application name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Required capabilities
    pub capabilities: Vec<CapabilityRequest>,
    /// Optional capabilities
    pub optional_capabilities: Vec<CapabilityRequest>,
    /// Entry point
    pub entry: String,
}

/// Capability request.
#[derive(Clone, Debug)]
pub struct CapabilityRequest {
    /// Capability type
    pub cap_type: String,
    /// Reason for needing it
    pub reason: String,
    /// Minimum permissions needed
    pub permissions: Permissions,
}
```

### Example Manifest

```toml
# app.toml

name = "example-app"
version = "1.0.0"
description = "An example application"
entry = "main.wasm"

[[capabilities]]
type = "storage"
reason = "Save user preferences"
permissions = { read = true, write = true, grant = false }

[[capabilities]]
type = "network"
reason = "Fetch updates"
permissions = { read = true, write = true, grant = false }

[[optional_capabilities]]
type = "console"
reason = "Debug output"
permissions = { read = false, write = true, grant = false }
```

## Capability Request Flow

```
     Application              ProcessManager            User/System
          │                         │                        │
          │  Request: "storage"     │                        │
          │  with reason            │                        │
          │────────────────────────▶│                        │
          │                         │                        │
          │                         │  (if policy requires   │
          │                         │   user approval)       │
          │                         │───────────────────────▶│
          │                         │                        │  User approves
          │                         │◀───────────────────────│
          │                         │                        │
          │                         │  Grant capability      │
          │  Capability granted     │                        │
          │  (slot 3)               │                        │
          │◀────────────────────────│                        │
```

## Application Sandbox

Applications are isolated by:

1. **Separate Process**: Each app runs in its own Web Worker/process
2. **Limited Capabilities**: Only granted capabilities are accessible
3. **Namespaced Storage**: Apps access only their storage namespace
4. **Network Policy**: Network access can be restricted per-app

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Application Sandbox                           │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Application Process                          ││
│  │                                                                ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        ││
│  │  │    Code      │  │    Heap      │  │    Stack     │        ││
│  │  │   (.wasm)    │  │  (malloc)    │  │              │        ││
│  │  └──────────────┘  └──────────────┘  └──────────────┘        ││
│  │                                                                ││
│  │  Capabilities: [storage-rw, network-r]                         ││
│  └────────────────────────────────────────────────────────────────┘│
│                              │                                      │
│                      IPC only│                                      │
│                              ▼                                      │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Runtime Services                             ││
│  │                                                                ││
│  │  Storage (namespaced)    Network (policy-limited)              ││
│  └────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

## WASM Applications

On WASM, applications are compiled to `wasm32-unknown-unknown`:

```rust
// my_app/src/lib.rs

#![no_std]
#![no_main]

extern crate alloc;
extern crate orbital_process;

use orbital_process::*;

#[no_mangle]
pub extern "C" fn _start() {
    debug("app: starting");
    
    // Get our process ID
    let pid = get_pid();
    debug(&format!("app: running as PID {}", pid));
    
    // Use granted capabilities
    // (capability slots are provided at startup)
    
    // Main application loop
    loop {
        // Do application work
        
        yield_now();
    }
}
```

## Documentation Structure (Future)

```
07-applications/
├── README.md           (this file)
├── 01-manifest.md     (Manifest format)
├── 02-lifecycle.md    (Lifecycle management)
├── 03-sandbox.md      (Sandbox details)
└── 04-sdk.md          (Application SDK)
```
