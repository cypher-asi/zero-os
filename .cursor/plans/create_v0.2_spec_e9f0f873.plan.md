---
name: Create v0.2 Spec
overview: Create an updated specification (v0.2) that reflects the actual implementation learnings while maintaining support for QEMU and bare metal targets. This spec will serve as a reference for auditing the codebase against the intended architecture.
todos:
  - id: create-v02-readme
    content: Create v0.2 README.md with architecture overview and section index
    status: pending
  - id: update-hal-spec
    content: Create 01-hal/ section with updated HAL traits and WASM-specific documentation
    status: pending
  - id: update-axiom-spec
    content: Create 02-axiom/ section documenting AxiomGateway, CommitLog, SysLog as implemented
    status: pending
  - id: update-kernel-spec
    content: Create 03-kernel/ section with updated syscall ABI and capability model
    status: pending
  - id: update-init-spec
    content: Create 04-init/ section with bootstrap and service discovery
    status: pending
  - id: update-runtime-spec
    content: Create 05-runtime/ section including console I/O model
    status: pending
  - id: update-apps-spec
    content: Create 07-applications/ section with ZeroApp trait and app protocol
    status: pending
  - id: update-desktop-spec
    content: Create 08-desktop/ section documenting zos-desktop architecture
    status: pending
  - id: create-supervisor-spec
    content: Create new 09-supervisor/ section documenting the web supervisor
    status: pending
  - id: add-audit-checklists
    content: Add compliance checklists to each section for auditing
    status: pending
---

# Zero OS v0.1.1 Specification

## Overview

Create a new specification under `docs/spec/v0.1.1/` that reflects the actual coded implementation while maintaining the architectural vision for QEMU and bare metal targets. The spec will be structured similarly to v0.1 but updated with implementation learnings.

## Key Changes from v0.1

### Architectural Evolution Learned from Implementation

1. **Supervisor/Desktop Separation**: The implementation split `zos-supervisor-web` into two crates - `zos-supervisor-web` (process/IPC management) and `zos-desktop` (compositor/window management). The spec should formalize this separation.

2. **Console I/O Model**: Evolved from IPC-based to syscall-based (`SYS_CONSOLE_WRITE`). The supervisor drains console output and routes to process-specific callbacks.

3. **Window Interaction**: Implemented universal drag threshold detection (5px) allowing click-to-interact and drag-to-move on all windows. This is a better UX than the original `content_interactive` flag approach.

4. **Kernel Size**: Current implementation (~5300 LOC) exceeds the 3000 LOC formal verification target. The spec should acknowledge pragmatic trade-offs and see areas to reduce.

5. **WASM-First Architecture**: The working implementation is WASM-first with Web Workers as processes. QEMU/bare metal remain future targets, but we still want to design the system such that can be supported without major changes. 
6. Axiom: Nothing directly interacts with Kernel except the Axiom. The Axiom is the verification layer that all syscalls and data must pass through.

## Specification Structure

### Section 00: Boot

- Minimal changes from v0.1
- Document WASM bootstrap sequence via JavaScript supervisor

### Section 01: HAL ([01-hal/README.md](docs/spec/v0.1/01-hal/README.md))

- Update `HAL` trait to match actual implementation in [zos-hal/src/lib.rs](crates/zos-hal/src/lib.rs)
- Document WASM HAL with SharedArrayBuffer syscall mechanism
- Keep QEMU/bare metal sections for future phases

### Section 02: Axiom ([02-axiom/README.md](docs/spec/v0.1/02-axiom/README.md))

- Document actual `AxiomGateway` from [zos-axiom/src/gateway.rs](crates/zos-axiom/src/gateway.rs)
- Update CommitType variants to match implementation
- Document IndexedDB persistence model for WASM

### Section 03: Kernel ([03-kernel/README.md](docs/spec/v0.1/03-kernel/README.md))

- Update syscall numbers to match canonical ABI in [zos-process/src/lib.rs](crates/zos-process/src/lib.rs)
- Document actual capability model from [zos-kernel/src/lib.rs](crates/zos-kernel/src/lib.rs)
- Update LOC estimates with realistic targets
- Document WASM-specific syscall flow (SharedArrayBuffer polling)

### Section 04: Init

- Document bootstrap sequence as implemented
- Update service discovery protocol with actual message tags
- Document supervisor's privileged kernel APIs

### Section 05: Runtime Services

- Update to reflect current service architecture
- Document console I/O model (SYS_CONSOLE_WRITE syscall)
- Update permission protocol documentation

### Section 06: Drivers

- Placeholder for QEMU/bare metal phases
- No changes for WASM target

### Section 07: Applications ([07-applications/README.md](docs/spec/v0.1/07-applications/README.md))

- Update `ZeroApp` trait from [zos-apps/src/app.rs](crates/zos-apps/src/app.rs)
- Document app protocol tags from [zos-apps/src/app_protocol/](crates/zos-apps/src/app_protocol/)
- Update `AppRuntime` documentation

### Section 08: Desktop (Major Update)

- Document `zos-desktop` architecture from [zos-desktop/src/lib.rs](crates/zos-desktop/src/lib.rs)
- New subsections for:
  - DesktopEngine core
  - Window management with drag threshold
  - Viewport/camera system
  - Input routing
  - Transition system
  - Persistence/snapshots
- Document React surface integration model
- Document supervisor separation (supervisor handles processes, desktop handles UI)

### Section 09: Supervisor (New Section)

- Document `Supervisor` from [zos-supervisor-web/src/supervisor/mod.rs](crates/zos-supervisor-web/src/supervisor/mod.rs)
- Process management via Web Workers
- Syscall polling and dispatch
- Console callback routing
- Separation of concerns from desktop

## Files to Create

```
docs/spec/v0.2/
├── README.md (overview, architecture diagram)
├── 00-boot/
│   └── README.md
├── 01-hal/
│   ├── README.md
│   ├── 01-targets.md
│   ├── 02-wasm-hal.md (new - detailed WASM implementation)
│   └── 03-traits.md
├── 02-axiom/
│   ├── README.md
│   ├── 01-syslog.md
│   ├── 02-commitlog.md
│   └── 03-replay.md
├── 03-kernel/
│   ├── README.md
│   ├── 01-processes.md
│   ├── 02-capabilities.md
│   ├── 03-ipc.md
│   └── 04-syscalls.md (updated ABI)
├── 04-init/
│   └── README.md
├── 05-runtime/
│   ├── README.md
│   └── 01-console-io.md (new)
├── 07-applications/
│   ├── README.md
│   ├── 01-zeroapp.md
│   └── 02-protocol.md
├── 08-desktop/
│   ├── README.md
│   ├── 01-engine.md
│   ├── 02-windows.md
│   ├── 03-input.md (includes drag threshold)
│   ├── 04-viewport.md
│   └── 05-persistence.md
└── 09-supervisor/
    └── README.md
```

## Audit Checklist Section

Each spec file will include a "Compliance Checklist" section to facilitate auditing:

- Which source files implement the spec
- Key invariants to verify
- Known deviations or TODOs
- Include what is different from the original v0.1.0 spec
- Look for naming inconsistencies. prefix should be zos for code and modules. 

## Approach

1. Start with the README.md providing overall architecture
2. Work through sections in dependency order (HAL → Axiom → Kernel → ...)
3. Cross-reference actual code with spec prose
4. Flag areas where implementation differs from ideal design