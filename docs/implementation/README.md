# Orbital OS Implementation Plan

> **Phased approach to building a capability-based, formally verifiable microkernel with deterministic replay.**

## Overview

This implementation plan breaks down Orbital OS development into three major phases, each targeting a different platform. The strategy is to get **end-to-end vertical slices working first**, then deepen functionality, ensuring **core invariants hold at every stage**.

### Three-Phase Strategy

```
Phase 1: WASM          Phase 2: QEMU          Phase 3: Bare Metal
(Browser)              (Virtual Hardware)      (Real Hardware)
    │                       │                       │
    ├─→ Prove concepts      ├─→ Add hardware       ├─→ Production ready
    ├─→ Fast iteration      ├─→ Real OS features   ├─→ Real devices
    ├─→ Easy debugging      ├─→ Virtual devices    ├─→ Performance
    └─→ Core invariants     └─→ Maintain invariants└─→ Security
```

| Phase | Platform | Focus | Stages | Duration |
|-------|----------|-------|--------|----------|
| [**Phase 1**](phase-1-wasm/README.md) | **WASM (Browser)** | Core architecture, capability system, IPC, deterministic replay | 7 stages | 4-6 weeks |
| [**Phase 2**](phase-2-qemu/README.md) | **QEMU (Virtual)** | VMM, preemptive scheduling, interrupts, virtual devices | 7 stages | 4-6 weeks |
| [**Phase 3**](phase-3-baremetal/README.md) | **Bare Metal** | Real hardware, device drivers, SMP, security, production readiness | 7 stages | 6-8 weeks |

**Total Estimated Time**: 14-20 weeks (3.5-5 months)

## Core Principles

### 1. Vertical Slices First

Always get end-to-end functionality working before deepening:

✅ **Good**: Minimal kernel → debug syscall → test pass → add next syscall
❌ **Bad**: Implement all syscalls before testing any

### 2. Test Before Advancing

Each stage has clear test criteria. **Do not proceed** until all tests pass.

### 3. Core Invariants Always True

These properties must hold at **every stage** of every phase:

#### Invariant 1: Two-Log Model
- SysLog records all syscalls (request + response)
- CommitLog records state mutations
- All syscalls flow through Axiom gateway

#### Invariant 2: Capability Integrity
- Capabilities only created by kernel
- Derived capabilities have permissions ≤ source
- Capability checks before every privileged operation

#### Invariant 3: Deterministic Replay
- `reduce(genesis, commits) -> state` is pure
- Same CommitLog always produces same state
- Replay must work across all three platforms

#### Invariant 4: Sender Verification
- Sender ID verified from trusted context
- Processes cannot spoof their identity
- Kernel trusts Axiom's sender verification

#### Invariant 5: Memory Isolation
- Processes cannot access other processes' memory
- Enforced by platform (WASM runtime, hardware MMU)
- Capability checks enforce access control

### 4. Follow Rust Conventions

All code must follow [`.cursor/cursor_rules_rust.md`](../../.cursor/cursor_rules_rust.md):

- Compiles with **no warnings**
- `cargo fmt` for formatting
- `cargo clippy -- -D warnings` for linting
- All tests pass (`cargo test`)
- No `unwrap`/`expect` in production code
- Use `thiserror` for library errors

## Phase Breakdown

### Phase 1: WASM (Browser-Hosted)

**Duration**: 4-6 weeks
**Platform**: Web Workers, IndexedDB, JavaScript APIs
**Goal**: Prove core architecture works

| Stage | Name | Duration | Key Deliverable |
|-------|------|----------|-----------------|
| 1.1 | Minimal Kernel + Debug | 2-3 days | Debug syscall works |
| 1.2 | Axiom Layer | 3-5 days | SysLog + CommitLog infrastructure |
| 1.3 | Capabilities + IPC | 5-7 days | Capability system, message passing |
| 1.4 | Process Management | 5-7 days | Multiple Web Worker processes |
| 1.5 | Init + Services | 5-7 days | Bootstrap, service discovery |
| 1.6 | Replay + Testing | 3-5 days | Deterministic replay verified |
| 1.7 | Web UI | 3-5 days | Browser inspection interface |

**Phase 1 Deliverables**:
- ✅ Kernel runs in browser
- ✅ Multiple processes communicating via IPC
- ✅ Capability-based access control working
- ✅ Axiom logging (SysLog + CommitLog)
- ✅ Deterministic replay demonstrated
- ✅ Web UI for inspection

[See full Phase 1 plan →](phase-1-wasm/README.md)

### Phase 2: QEMU (Virtual Hardware)

**Duration**: 4-6 weeks
**Platform**: QEMU x86_64, virtual devices
**Goal**: Add real OS features

| Stage | Name | Duration | Key Deliverable |
|-------|------|----------|-----------------|
| 2.1 | Bootloader + Serial | 3-4 days | Boot in QEMU, serial output |
| 2.2 | VMM + Paging | 5-7 days | Virtual memory, page tables |
| 2.3 | Interrupts + Timer | 5-7 days | Preemptive scheduling |
| 2.4 | Port Kernel | 5-7 days | Phase 1 kernel on QEMU |
| 2.5 | VirtIO Block | 3-5 days | Persistent storage |
| 2.6 | Init + Services | 3-5 days | Services on QEMU |
| 2.7 | Replay + Persistence | 3-5 days | Replay from disk |

**Phase 2 Deliverables**:
- ✅ Boots in QEMU
- ✅ Virtual memory working
- ✅ Preemptive scheduling
- ✅ VirtIO devices (block storage)
- ✅ CommitLog persists to disk
- ✅ Replay from disk after reboot
- ✅ All Phase 1 tests pass on QEMU

[See full Phase 2 plan →](phase-2-qemu/README.md)

### Phase 3: Bare Metal (Real Hardware)

**Duration**: 6-8 weeks
**Platform**: Real x86_64/ARM64 hardware
**Goal**: Production-ready OS

| Stage | Name | Duration | Key Deliverable |
|-------|------|----------|-----------------|
| 3.1 | UEFI Boot | 5-7 days | Boot on real hardware |
| 3.2 | NVMe Driver | 5-7 days | Real storage working |
| 3.3 | Network Driver | 5-7 days | Network connectivity |
| 3.4 | SMP Multi-core | 5-7 days | Multiple CPU cores |
| 3.5 | ACPI + Power | 3-5 days | Power management |
| 3.6 | Security | 5-7 days | TPM, secure boot |
| 3.7 | Production Ready | 5-7 days | Stability, performance |

**Phase 3 Deliverables**:
- ✅ Boots on real hardware via UEFI
- ✅ NVMe and network drivers
- ✅ Multi-core support (SMP)
- ✅ Power management (ACPI)
- ✅ Security (TPM, secure boot)
- ✅ Production-ready stability
- ✅ Performance: 1M syscalls/sec per core

[See full Phase 3 plan →](phase-3-baremetal/README.md)

## Development Workflow

### Per-Stage Process

For each stage:

1. **Read** the stage document (e.g., `phase-1-wasm/stage-1.1-minimal-kernel.md`)
2. **Understand** the goals and test criteria
3. **Implement** the features listed
4. **Test** each feature as you build it
5. **Verify** invariants still hold
6. **Run** all automated tests
7. **Document** any deviations or issues
8. **Move to next stage** only when all tests pass

### Daily Workflow

```bash
# 1. Start of day: Review progress
make status

# 2. Read current stage document
cat docs/implementation/phase-X/stage-X.Y-*.md

# 3. Implement features
code crates/orbital-*/src/

# 4. Format and lint
cargo fmt
cargo clippy -- -D warnings

# 5. Run tests
cargo test --all

# 6. Manual testing (as needed)
make serve          # Phase 1
make qemu           # Phase 2
make boot-usb       # Phase 3

# 7. Commit progress
git add .
git commit -m "Stage X.Y: Implement feature Z"
```

### Code Review Checklist

Before considering a stage complete:

- [ ] All code compiles without warnings
- [ ] `cargo fmt` applied
- [ ] `cargo clippy` passes with `-D warnings`
- [ ] All automated tests pass
- [ ] Manual test criteria met
- [ ] Core invariants verified
- [ ] Code documented (public APIs)
- [ ] No `unwrap`/`expect` in production paths
- [ ] Errors include context

## Project Structure

```
orbital-os/
├── docs/
│   ├── spec/                    # System specification
│   │   ├── 00-boot/
│   │   ├── 01-hal/
│   │   ├── 02-axiom/            # Verification layer
│   │   ├── 03-kernel/           # Microkernel
│   │   ├── 04-init/             # Init process
│   │   └── 05-runtime/          # Runtime services
│   ├── implementation/          # THIS DIRECTORY
│   │   ├── phase-1-wasm/        # Phase 1 plan + stages
│   │   ├── phase-2-qemu/        # Phase 2 plan + stages
│   │   └── phase-3-baremetal/   # Phase 3 plan + stages
│   └── whitepaper/              # Architecture docs
├── crates/
│   ├── orbital-hal/             # Hardware abstraction
│   ├── orbital-axiom/           # Verification layer (Phase 1.2+)
│   ├── orbital-kernel/          # Microkernel
│   ├── orbital-process/         # Process library
│   ├── orbital-init/            # Init process (Phase 1.5+)
│   ├── orbital-apps/            # Userspace apps (Terminal, Clock, Calculator, PermissionManager)
│   ├── orbital-boot/            # Bootloader (Phase 2.1+)
│   └── orbital-drivers/         # Device drivers (Phase 3.2+)
├── apps/
│   └── orbital-web/             # Browser UI (Phase 1)
├── tools/
│   └── dev-server/              # Development server
├── Makefile                     # Build targets
└── Cargo.toml                   # Workspace config
```

## Common Makefile Targets

```bash
# Phase 1 (WASM)
make build-wasm      # Build WASM modules
make serve           # Start dev server
make test-wasm       # Run WASM tests

# Phase 2 (QEMU)
make bootimage       # Create bootable image
make qemu            # Run in QEMU
make qemu-debug      # Run with GDB server
make test-qemu       # Run QEMU integration tests

# Phase 3 (Bare Metal)
make uefi-image      # Create UEFI bootable image
make boot-usb        # Flash to USB drive
make test-hardware   # Run hardware tests

# All phases
make fmt             # Format all code
make clippy          # Lint all code
make test            # Run all tests
make clean           # Clean build artifacts
make status          # Show current implementation status
```

## Success Metrics

### Phase 1 Complete

- [ ] All 7 stages complete
- [ ] Kernel boots in browser
- [ ] IPC and capabilities working
- [ ] Deterministic replay demonstrated
- [ ] Web UI functional
- [ ] All tests passing
- [ ] Core invariants verified

### Phase 2 Complete

- [ ] All 7 stages complete
- [ ] Boots in QEMU
- [ ] Virtual memory working
- [ ] Preemptive scheduling working
- [ ] VirtIO storage working
- [ ] Replay from disk working
- [ ] Phase 1 tests pass on QEMU
- [ ] Core invariants verified

### Phase 3 Complete

- [ ] All 7 stages complete
- [ ] Boots on real hardware
- [ ] NVMe and network working
- [ ] Multi-core support
- [ ] Security features enabled
- [ ] 24-hour stress test passes
- [ ] Performance targets met
- [ ] Core invariants verified
- [ ] **Orbital OS is production-ready**

## Risk Mitigation

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Deterministic replay breaks | High | Test replay at every stage |
| Performance issues | Medium | Profile early, optimize hot paths |
| Hardware compatibility | Medium | Test on multiple platforms |
| Security vulnerabilities | High | Security audit, fuzzing |

### Schedule Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Stages take longer than estimated | Medium | Time-box stages, cut scope if needed |
| Blocked on dependencies | Low | Start with WASM (no hardware deps) |
| Scope creep | Medium | Strictly follow stage plans |

## Getting Help

- **Specs**: See `docs/spec/` for detailed specifications
- **Rust conventions**: See `.cursor/cursor_rules_rust.md`
- **Questions**: Open GitHub issue with `[implementation]` tag
- **Bugs**: File issues with stage number and test failure

## Related Documentation

- [Specification](../spec/README.md) - System architecture and design
- [Whitepaper](../whitepaper/) - Background and motivation
- [Rust Conventions](../../.cursor/cursor_rules_rust.md) - Code quality rules

## Let's Build!

**Current Status**: Ready to start Phase 1, Stage 1.1

**Next Step**: [Begin Phase 1 →](phase-1-wasm/README.md)

**First Task**: [Stage 1.1: Minimal Kernel + Debug](phase-1-wasm/stage-1.1-minimal-kernel.md)

---

*This implementation plan is a living document. It will be updated as we learn and adapt during development.*
