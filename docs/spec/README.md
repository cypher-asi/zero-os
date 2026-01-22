# Zero OS Specification v5.0

> A capability-based, formally verifiable microkernel with deterministic replay.

## Core Principles

1. **Two-Log Model**: SysLog records all syscalls (audit), CommitLog records state changes (replay).
2. **Deterministic Replay**: Same CommitLog always produces same state.
3. **Capability-Only Access**: All resources accessed through unforgeable capability tokens.
4. **Formally Verifiable Kernel**: Total kernel code under 3,000 LOC.
5. **WASM-First Architecture**: Primary target is browser-hosted WASM with upgrade path to native.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Layer 8: Desktop/Compositor                           [08-desktop/]        │
│           Window management, input routing, visual shell                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 7: Applications                                 [07-applications/]   │
│           Sandboxed user applications, app model                            │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 6: Drivers                                      [06-drivers/]        │
│           User-space device drivers (block, network, display)               │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 5: Runtime Services                             [05-runtime/]        │
│           Process manager, permissions, identity, storage, network          │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 4: Init Process                                 [04-init/]           │
│           Bootstrap, service supervision                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 3: Microkernel                                  [03-kernel/]         │
│           Capabilities, threads, VMM, IPC, interrupts                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 2: Axiom (Verification)                         [02-axiom/]          │
│           SysLog (audit), CommitLog (replay), sender verification           │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 1: Hardware Abstraction Layer                   [01-hal/]            │
│           Platform-specific: WASM/QEMU/Bare Metal                           │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 0: Boot                                         [00-boot/]           │
│           Reset vector, early init (WASM: handled by browser)               │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Syscall Flow

Every system call flows through Axiom for verification and logging:

```
   User Process
        │
        ▼
   ┌─────────┐
   │ Syscall │
   │ ABI     │
   └────┬────┘
        │
        ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                              AXIOM                                        │
│                                                                           │
│  1. Verify sender from trusted context (cannot be spoofed)                │
│  2. Create SysEvent (request)                                             │
│  3. Append to SysLog (audit trail)                                        │
│                                                                           │
└─────────────────────────────────┬─────────────────────────────────────────┘
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                              KERNEL                                       │
│                                                                           │
│  4. Check capability in caller's CSpace                                   │
│  5. Execute operation                                                     │
│  6. Emit Commit(s) for state changes (if successful)                      │
│                                                                           │
│      ┌──────────┐    ┌──────────┐    ┌──────────┐                        │
│      │  DENY    │    │  GRANT   │────│  Commits │                        │
│      └────┬─────┘    └────┬─────┘    └──────────┘                        │
│           │               │                                               │
└───────────┼───────────────┼───────────────────────────────────────────────┘
            │               │
            ▼               ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                              AXIOM                                        │
│                                                                           │
│  7. Append Commits to CommitLog (hash-chained)                            │
│  8. Create SysEvent (response)                                            │
│  9. Append to SysLog                                                      │
│                                                                           │
└─────────────────────────────────┬─────────────────────────────────────────┘
                                  │
                                  ▼
                           Return result
```

## Two-Log Model

| Log | Purpose | Contents | Used for Replay |
|-----|---------|----------|-----------------|
| **SysLog** | Audit trail | All syscall requests + responses | No |
| **CommitLog** | State mutations | Successful state changes only | Yes |

**Key insight:** A SysEvent may cause zero, one, or many Commits:

| SysEvent | Commits Generated |
|----------|-------------------|
| `CapGrant` (success) | `CapInserted` |
| `CapGrant` (failure) | None |
| `Spawn` | `ProcessCreated`, `CapInserted` (multiple) |
| `Exit` | `ProcessExited`, `CapRemoved` (cleanup) |

## Platform Capabilities Matrix

| Feature              | WASM (Browser)    | QEMU (Phase 2)    | Bare Metal (Phase 7) |
|----------------------|-------------------|-------------------|----------------------|
| **Process Isolation**| Web Workers       | Hardware VMM      | Hardware MMU         |
| **Memory Model**     | Linear memory     | Virtual memory    | Physical + virtual   |
| **Scheduling**       | Cooperative       | Preemptive        | Preemptive           |
| **Timer**            | `performance.now` | PIT/HPET          | HPET/TSC             |
| **Entropy**          | `crypto.random`   | virtio-rng        | RDRAND/TPM           |
| **Storage**          | IndexedDB         | virtio-blk        | NVMe/SATA            |
| **Network**          | Fetch API         | virtio-net        | NIC drivers          |
| **Interrupts**       | N/A (async)       | APIC/IOAPIC       | APIC/MSI-X           |
| **Debug**            | `console.log`     | Serial/VGA        | Serial/VGA           |

## Type Ownership Table

| Type            | Crate            | Description                         |
|-----------------|------------------|-------------------------------------|
| `ProcessId`     | `Zero-kernel` | Process identifier                  |
| `EndpointId`    | `Zero-kernel` | IPC endpoint identifier             |
| `CapSlot`       | `Zero-kernel` | Index into capability space         |
| `Capability`    | `Zero-kernel` | Unforgeable authority token         |
| `Message`       | `Zero-kernel` | IPC message structure               |
| `SysEvent`      | `Zero-axiom`  | Syscall request/response            |
| `Commit`        | `Zero-axiom`  | State mutation event                |
| `HAL`           | `Zero-hal`    | Platform abstraction trait          |

## Reading Order

For implementers, the recommended reading order is:

1. **[02-axiom/README.md](02-axiom/README.md)** - Axiom verification layer (SysLog + CommitLog)
2. **[02-axiom/02-commitlog.md](02-axiom/02-commitlog.md)** - Commit types and hash chain
3. **[02-axiom/03-replay.md](02-axiom/03-replay.md)** - State reconstruction
4. **[03-kernel/README.md](03-kernel/README.md)** - Kernel overview and verification goals
5. **[03-kernel/03-capabilities.md](03-kernel/03-capabilities.md)** - Capability system
6. **[03-kernel/06-syscalls.md](03-kernel/06-syscalls.md)** - Syscall ABI
7. **[01-hal/03-traits.md](01-hal/03-traits.md)** - HAL interface
8. **[03-kernel/01-threads.md](03-kernel/01-threads.md)** - Thread model
9. **[03-kernel/04-ipc.md](03-kernel/04-ipc.md)** - IPC system
10. **[04-init/01-bootstrap.md](04-init/01-bootstrap.md)** - Bootstrap and state reconstruction
11. **[05-runtime/](05-runtime/)** - Runtime services

## Spec File Conventions

- Each specification file uses the following structure:
  - **Overview**: High-level description
  - **Data Structures**: Rust types with doc comments
  - **Operations**: Functions and their semantics
  - **Invariants**: Properties that must always hold
  - **WASM Notes**: Platform-specific considerations

- Code examples are provided in Rust with `#![no_std]` compatibility.

- All types use strong typing (newtypes for IDs, enums for states).

## Version History

| Version | Date       | Description                              |
|---------|------------|------------------------------------------|
| 5.0     | 2026-01-17 | Axiom layer redesign: SysLog + CommitLog |
| 4.0     | 2026-01-17 | New spec from scratch with Axiom-gated architecture |
