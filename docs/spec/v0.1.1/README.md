# Zero OS v0.1.1 Specification

> A capability-based microkernel operating system with deterministic replay.

## Overview

Zero OS is a WASM-first operating system designed around three core principles:

1. **Capability-Based Security**: All resource access is mediated by unforgeable capability tokens
2. **Deterministic Replay**: All state mutations are recorded in a CommitLog for deterministic replay
3. **Formal Verification Path**: Minimal kernel surface area with explicit verification targets

This specification documents the **actual implementation** of Zero OS v0.1.1, incorporating learnings from the initial WASM-based prototype while maintaining the architectural vision for future QEMU and bare metal targets.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Browser / Host                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌───────────────────────┐         ┌───────────────────────────────────┐  │
│   │      React Surface    │ <-----> │        DesktopController          │  │
│   │  (Desktop, Windows)   │         │     (zos-desktop WASM pkg)        │  │
│   └───────────────────────┘         └───────────────────────────────────┘  │
│                                                        │                    │
│                                                        │                    │
│   ┌───────────────────────────────────────────────────┴────────────────┐   │
│   │                         Supervisor                                  │   │
│   │                    (zos-supervisor)                            │   │
│   │   • Process lifecycle (spawn/kill)                                 │   │
│   │   • Syscall polling & dispatch                                     │   │
│   │   • Console callback routing                                       │   │
│   └──────────────────────────────┬─────────────────────────────────────┘   │
│                                  │                                          │
│   ┌──────────────────────────────▼─────────────────────────────────────┐   │
│   │                         Kernel (zos-kernel)                         │   │
│   │   ┌─────────────────────────────────────────────────────────────┐  │   │
│   │   │                   Axiom Gateway                              │  │   │
│   │   │   • SysLog (audit trail of all syscalls)                    │  │   │
│   │   │   • CommitLog (state mutations for replay)                  │  │   │
│   │   └─────────────────────────────────────────────────────────────┘  │   │
│   │                                                                     │   │
│   │   • Process table                                                   │   │
│   │   • Capability spaces (CSpace per process)                         │   │
│   │   • IPC endpoints & message queues                                 │   │
│   │   • Syscall handlers                                               │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                  │                                          │
│   ┌──────────────────────────────▼─────────────────────────────────────┐   │
│   │                     HAL - Web Workers                               │   │
│   │   • Process = Web Worker                                           │   │
│   │   • Syscalls via SharedArrayBuffer polling                         │   │
│   │   • Memory = WASM linear memory (64KB pages)                       │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Key Changes from v0.1.0

### 1. Supervisor/Desktop Separation

The original `zos-supervisor` has been split into two distinct concerns:

- **zos-supervisor**: Process management, syscall dispatch, IPC routing
- **zos-desktop**: Window compositor, input routing, viewport management

This separation enables:
- Independent testing of desktop logic
- Clear boundary between OS primitives and UI
- Potential for alternative UIs (headless, CLI, etc.)

### 2. Console I/O Model

Console output evolved from IPC-based messaging to a dedicated syscall:

```rust
// Old: Required endpoint capability
send(console_endpoint_slot, MSG_CONSOLE_OUTPUT, data);

// New: Direct syscall, supervisor drains via callback
SYS_CONSOLE_WRITE (0x07)
```

The supervisor drains console output and routes to per-process callbacks, enabling proper terminal isolation.

### 3. Axiom Gateway Pattern

All syscalls flow through the Axiom Gateway which:
1. Logs the request to SysLog (audit)
2. Executes the kernel operation
3. Records any state mutations to CommitLog (replay)
4. Logs the response to SysLog

This ensures every operation is auditable and replayable.

### 4. Universal Drag Threshold

Window interaction uses a 5px drag threshold allowing both click-to-interact and drag-to-move on all windows. This provides better UX than the original `content_interactive` flag approach.

## Specification Sections

| Section | Description | Status |
|---------|-------------|--------|
| [00-boot](./00-boot/README.md) | Bootstrap sequence | Documented |
| [01-hal](./01-hal/README.md) | Hardware Abstraction Layer | Documented |
| [02-axiom](./02-axiom/README.md) | Axiom verification layer | Documented |
| [03-kernel](./03-kernel/README.md) | Kernel syscalls and capabilities | Documented |
| [04-init](./04-init/README.md) | Init process and service discovery | Documented |
| [05-runtime](./05-runtime/README.md) | Runtime services and console I/O | Documented |
| [07-applications](./07-applications/README.md) | ZeroApp trait and app protocol | Documented |
| [08-desktop](./08-desktop/README.md) | Desktop compositor | Documented |
| [09-supervisor](./09-supervisor/README.md) | Web supervisor | Documented |

## Crate Map

| Crate | Purpose | LOC (approx) |
|-------|---------|--------------|
| `zos-hal` | Hardware abstraction trait | ~220 |
| `zos-axiom` | Verification layer (SysLog, CommitLog) | ~800 |
| `zos-kernel` | Core kernel (processes, caps, IPC) | ~5300 |
| `zos-process` | Process-side syscall library | ~1000 |
| `zos-init` | Init process (PID 1) | ~400 |
| `zos-apps` | ZeroApp trait and app protocol | ~600 |
| `zos-desktop` | Window compositor | ~2500 |
| `zos-supervisor` | Web supervisor | ~800 |

## Design Principles

1. **Everything through Axiom**: No direct kernel interaction—all syscalls flow through the Axiom gateway for auditing and replay.

2. **Capability-based access**: All resources (endpoints, console, storage) are accessed via capabilities in the process's CSpace.

3. **Process isolation**: Each process runs in its own Web Worker with isolated linear memory.

4. **Deterministic replay**: The CommitLog records all state mutations; replaying commits produces identical state.

5. **Pure Rust core**: Desktop and kernel logic is pure Rust, testable without browser dependencies.

## Compliance Checklist

Each specification section includes a compliance checklist for auditing:

- Source files implementing the spec
- Key invariants to verify
- Known deviations or TODOs
- Differences from v0.1.0

## Future Targets

While v0.1.1 is WASM-first, the architecture supports:

- **Phase 2**: QEMU virtual machine target
- **Phase 3**: Bare metal x86_64 target

The HAL trait abstracts platform differences, allowing the kernel to run unchanged across targets.
