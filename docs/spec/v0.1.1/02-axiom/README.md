# 02 - Axiom Layer

## Overview

The Axiom layer is the verification foundation of Zero OS. **Nothing interacts directly with the kernel except through the Axiom Gateway.** This ensures:

1. **Audit Trail**: Every syscall is logged (request + response) to SysLog
2. **Deterministic Replay**: All state mutations are recorded in CommitLog
3. **Integrity Verification**: Hash chains detect tampering

## Core Guarantee

> **Same CommitLog always produces same state.**

This is the foundation of Zero OS's deterministic replay capability.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Axiom Gateway                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    syscall()                         │   │
│  │  1. Log request to SysLog                           │   │
│  │  2. Execute kernel operation                        │   │
│  │  3. Append commits to CommitLog                     │   │
│  │  4. Log response to SysLog                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌────────────────────┐    ┌─────────────────────────────┐ │
│  │      SysLog        │    │         CommitLog           │ │
│  │  • Event ID (u64)  │    │  • Sequence (u64)           │ │
│  │  • Sender PID      │    │  • Hash chain               │ │
│  │  • Timestamp       │    │  • State mutations          │ │
│  │  • Request/Response│    │  • Causal link to SysLog    │ │
│  └────────────────────┘    └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Components

| Component | Purpose | Spec |
|-----------|---------|------|
| [SysLog](./01-syslog.md) | Audit trail of all syscalls | Request + response pairs |
| [CommitLog](./02-commitlog.md) | State mutations for replay | Hash-chained commits |
| [Replay](./03-replay.md) | Deterministic state reconstruction | From commits |

## AxiomGateway

The central entry point for all syscalls:

```rust
pub struct AxiomGateway {
    syslog: SysLog,
    commitlog: CommitLog,
}

impl AxiomGateway {
    /// Process a syscall through Axiom
    pub fn syscall<F>(
        &mut self,
        sender: ProcessId,
        syscall_num: u32,
        args: [u32; 4],
        timestamp: u64,
        kernel_fn: F,
    ) -> (i64, Vec<CommitId>)
    where
        F: FnMut(u32, [u32; 4]) -> (i64, Vec<CommitType>);
}
```

### Syscall Flow

1. **Log Request**: Record syscall to SysLog with unique event ID
2. **Execute**: Call kernel function, which returns result and commits
3. **Append Commits**: Add each commit to CommitLog with causal link
4. **Log Response**: Record result to SysLog, linked to request

## Example Flow

```
Process P1 calls SYS_EP_CREATE
           │
           ▼
┌─────────────────────────────────────────┐
│ gateway.syscall(pid=1, num=0x35, ...)   │
└──────────────────┬──────────────────────┘
                   │
    ┌──────────────┴──────────────────────────────┐
    │                                              │
    ▼                                              │
┌──────────────────┐                               │
│ SysLog: Request  │                               │
│ • id: 0          │                               │
│ • sender: 1      │                               │
│ • syscall: 0x35  │                               │
│ • timestamp      │                               │
└──────────────────┘                               │
                   │                               │
                   ▼                               │
          kernel_fn(0x35, args)                    │
                   │                               │
                   ▼                               │
┌──────────────────┐                               │
│ CommitLog: Append│                               │
│ • seq: 1         │                               │
│ • type: EndpointCreated                          │
│ • caused_by: 0   │ ←────────────────────────────┘
│ • hash chain     │
└──────────────────┘
                   │
                   ▼
┌──────────────────┐
│ SysLog: Response │
│ • id: 1          │
│ • request_id: 0  │
│ • result: 0      │
└──────────────────┘
```

## Internal Operations

Some operations bypass SysLog but still record to CommitLog:

```rust
/// Append a commit directly (bypassing SysLog)
pub fn append_internal_commit(&mut self, commit_type: CommitType, timestamp: u64) -> CommitId;
```

Use cases:
- Timer-driven cleanup
- Internal state transitions
- Supervisor-initiated operations

## State Summary

```rust
pub struct GatewayState {
    pub syslog_len: usize,
    pub syslog_next_id: u64,
    pub commitlog_len: usize,
    pub commitlog_seq: u64,
    pub commitlog_head: CommitId,
}
```

## Compliance Checklist

### Source Files
- `crates/zos-axiom/src/lib.rs` - Module exports
- `crates/zos-axiom/src/gateway.rs` - AxiomGateway
- `crates/zos-axiom/src/syslog.rs` - SysLog
- `crates/zos-axiom/src/commitlog.rs` - CommitLog
- `crates/zos-axiom/src/replay.rs` - Replay utilities

### Key Invariants
- [ ] All syscalls flow through AxiomGateway
- [ ] SysLog events have monotonic IDs
- [ ] CommitLog maintains valid hash chain
- [ ] Each commit links to causing syscall (when applicable)
- [ ] Integrity verification passes after any operation

### Differences from v0.1.0
- Axiom is now the mandatory entry point for all kernel operations
- Added internal commit path for non-syscall operations
- GatewayState provides summary for monitoring/debugging
