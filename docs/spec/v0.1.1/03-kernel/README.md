# 03 - Kernel

## Overview

The Zero OS kernel is a capability-based microkernel providing:

- Process management and isolation
- Capability spaces (CSpace) for access control
- IPC via endpoints and message queues
- Syscall interface for user-space processes

All kernel operations flow through the Axiom Gateway for auditing and replay.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                           Kernel<H: HAL>                         │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                     Axiom Gateway                          │ │
│  │  • SysLog (audit)                                         │ │
│  │  • CommitLog (replay)                                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐  │
│  │ Process Table  │  │    CSpaces     │  │    Endpoints     │  │
│  │ • PID → Process│  │ • PID → Caps[] │  │ • ID → Queue     │  │
│  │ • State        │  │ • Permissions  │  │ • Owner          │  │
│  │ • Memory       │  │ • Object refs  │  │ • Messages       │  │
│  └────────────────┘  └────────────────┘  └──────────────────┘  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Syscall Handlers                        │   │
│  │  • Process: exit, spawn                                  │   │
│  │  • Capability: grant, revoke, inspect                    │   │
│  │  • IPC: send, receive, call, reply                       │   │
│  │  • Misc: debug, time, console                            │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

## Key Concepts

### Processes

Each process has:
- Unique PID (u64)
- Name for debugging
- State (Running, Waiting, Exited)
- Memory size tracking
- Associated HAL handle (Web Worker on WASM)

### Capability Spaces (CSpace)

Each process has a CSpace containing capability slots:

```
CSpace for PID 3 (Terminal):
┌──────┬──────────────────┬─────────────────────────┐
│ Slot │ Object Type      │ Permissions             │
├──────┼──────────────────┼─────────────────────────┤
│  0   │ Endpoint (1)     │ Read, Write             │
│  1   │ Endpoint (3)     │ Read, Write             │
│  2   │ Console          │ Write                   │
└──────┴──────────────────┴─────────────────────────┘
```

### IPC Endpoints

Endpoints are message queues with owner processes:

```rust
pub struct Endpoint {
    pub id: EndpointId,
    pub owner: ProcessId,
    pub queue: VecDeque<Message>,
}

pub struct Message {
    pub from_pid: ProcessId,
    pub tag: u32,
    pub data: Vec<u8>,
}
```

## Specification Sections

| Section | Description |
|---------|-------------|
| [01-processes.md](./01-processes.md) | Process lifecycle and state |
| [02-capabilities.md](./02-capabilities.md) | Capability model and CSpace |
| [03-ipc.md](./03-ipc.md) | Inter-process communication |
| [04-syscalls.md](./04-syscalls.md) | Syscall ABI reference |

## LOC Analysis

Current implementation: ~5300 LOC

| Component | LOC | Target | Notes |
|-----------|-----|--------|-------|
| Process management | ~800 | 500 | Could reduce with simpler state machine |
| Capability system | ~1200 | 800 | Core complexity, hard to reduce |
| IPC | ~1000 | 600 | Message queues add overhead |
| Syscall dispatch | ~1500 | 800 | Many syscalls, could consolidate |
| Axiom integration | ~500 | 400 | Necessary for replay |
| Tests | ~300 | Keep | Essential for verification |

**Path to 3000 LOC target:**
- Simplify process state machine
- Reduce syscall variants (consolidate similar operations)
- Extract non-critical functionality to user-space

## Compliance Checklist

### Source Files
- `crates/zos-kernel/src/lib.rs` - Main kernel module

### Key Invariants
- [ ] All syscalls flow through Axiom Gateway
- [ ] Capability checks before any resource access
- [ ] Process isolation (no shared memory)
- [ ] Deterministic behavior for replay

### Differences from v0.1.0
- Kernel LOC exceeds 3000 target (tracking for reduction)
- Console output via syscall (not IPC)
- Privileged kernel APIs for supervisor
- Revocation notifications to affected processes
