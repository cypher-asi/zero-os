# Phase 2: QEMU (Virtual Hardware)

> **Goal**: Port Zero OS to QEMU with virtual hardware (VMM, preemptive scheduling, interrupts).

## Overview

Phase 2 moves from browser to QEMU, adding real OS features:
- Hardware virtual memory (page tables)
- Preemptive scheduling (timer interrupts)
- Interrupt handling (APIC/IOAPIC)
- VirtIO devices (block, network)
- Serial console output

This phase proves the kernel can manage real hardware while maintaining invariants.

### Platform Transition

| Feature              | Phase 1 (WASM)     | Phase 2 (QEMU)      |
|----------------------|--------------------|---------------------|
| **Process Isolation**| Web Workers        | Hardware VMM        |
| **Memory Model**     | Linear memory      | Page tables         |
| **Scheduling**       | Cooperative        | Preemptive (timer)  |
| **Timer**            | `performance.now()`| PIT/HPET            |
| **Entropy**          | `crypto.random`    | virtio-rng          |
| **Storage**          | IndexedDB          | virtio-blk          |
| **Network**          | Fetch API          | virtio-net          |
| **Debug**            | console.log        | Serial port         |

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        QEMU Machine                              │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                   Zero Kernel                           │ │
│  │                                                           │ │
│  │  Axiom Layer → Kernel Core → HAL (x86_64)                 │ │
│  └───────────────────────────────────────────────────────────┘ │
│         │         │         │         │                         │
│         ▼         ▼         ▼         ▼                         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐              │
│  │ Process │ │ Process │ │ Process │ │ Process │              │
│  │ (init)  │ │(terminal)│ │(storage)│ │ (app)   │              │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘              │
│      │             │             │             │                │
│      └─────────────┴─────────────┴─────────────┘                │
│                       │                                         │
│                       ▼                                         │
│            ┌─────────────────────┐                              │
│            │  Hardware Devices   │                              │
│            │  • Serial           │                              │
│            │  • PIT/HPET         │                              │
│            │  • APIC/IOAPIC      │                              │
│            │  • VirtIO (block)   │                              │
│            └─────────────────────┘                              │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Stages

| Stage | Name | Focus | Test Criteria |
|-------|------|-------|---------------|
| [2.1](stage-2.1-bootloader-serial.md) | Bootloader + Serial | Boot to kernel, print to serial | "Hello from QEMU" appears |
| [2.2](stage-2.2-vmm-paging.md) | VMM + Paging | Virtual memory, page tables | Process has isolated address space |
| [2.3](stage-2.3-interrupts-timer.md) | Interrupts + Timer | IRQ handling, preemptive scheduling | Timer interrupt fires, context switch works |
| [2.4](stage-2.4-port-kernel.md) | Port Kernel | Adapt Phase 1 kernel to native | Syscalls work on QEMU |
| [2.5](stage-2.5-virtio-block.md) | VirtIO Block | Persistent storage driver | Can read/write to virtual disk |
| [2.6](stage-2.6-init-services.md) | Init + Services | Port Phase 1 services | Terminal service works |
| [2.7](stage-2.7-replay-persistence.md) | Replay + Persistence | CommitLog to disk, replay from disk | Survives reboot |

## Core Invariants (Still True)

All Phase 1 invariants must still hold:

### 1. Two-Log Model
- ✅ SysLog and CommitLog still maintained
- ✅ All syscalls flow through Axiom
- ✅ Commits recorded for state changes

### 2. Capability Integrity
- ✅ Capabilities cannot be forged
- ✅ Capability checks before operations
- ✅ Hardware memory protection enforces isolation

### 3. Deterministic Replay
- ✅ Same CommitLog produces same state
- ✅ Timer interrupts don't affect determinism (not recorded in CommitLog)
- ✅ Replay works identically on QEMU and WASM

### 4. Sender Verification
- ✅ Sender ID from trusted CPU context (CR3, process register)
- ✅ Cannot be spoofed by process

### 5. Memory Isolation
- ✅ Hardware page tables enforce memory boundaries
- ✅ Processes cannot access other processes' memory
- ✅ Kernel memory protected from user processes

## New Challenges

### 1. Boot Sequence

Unlike WASM (where browser handles boot), we must:
- Write bootloader
- Set up GDT, IDT
- Enable paging
- Jump to kernel

### 2. Preemptive Scheduling

- Timer interrupt preempts running process
- Context switch: save/restore registers
- Thread scheduler decides next process
- Cooperative vs preemptive: must handle both

### 3. Interrupt Handling

- Register IRQ handlers
- ACK interrupts (APIC EOI)
- Mask/unmask interrupts
- Nested interrupts handling

### 4. Virtual Memory

- Page table setup (4-level on x86_64)
- Map kernel at high address
- Per-process page tables
- Page fault handling

## Development Workflow

### Building for QEMU

```bash
# Build kernel
cargo build --release --target x86_64-unknown-none

# Create bootable image
make bootimage

# Run in QEMU
make qemu
```

### Debugging

```bash
# QEMU with GDB server
make qemu-debug

# Connect GDB
gdb target/x86_64-unknown-none/release/Zero-kernel
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
```

### Testing

- Unit tests (run on host)
- Integration tests (run in QEMU)
- Determinism tests (replay from CommitLog)

## Dependencies

### Rust Crates

```toml
[dependencies]
# Hardware abstractions
x86_64 = "0.14"
uart_16550 = "0.2"
pic8259 = "0.10"
volatile = "0.4"

# Boot
bootloader = { version = "0.11", features = ["map_physical_memory"] }

# Existing
Zero-hal = { workspace = true }
Zero-axiom = { workspace = true }
Zero-kernel = { workspace = true }
```

### Build Tools

- `bootimage` - Create bootable disk images
- `qemu-system-x86_64` - Emulator
- `gdb` - Debugger

## File Structure

```
crates/
  Zero-hal/
    src/
      x86_64.rs            # x86_64 HAL implementation
      x86_64/
        serial.rs
        interrupts.rs
        vmm.rs
        apic.rs
  Zero-boot/            # Bootloader/early init
    src/
      boot.rs
      gdt.rs
      idt.rs

target/
  x86_64-unknown-none/     # Target triple for bare metal
```

## Success Criteria for Phase 2

Phase 2 is complete when:

1. ✅ Kernel boots in QEMU from disk image
2. ✅ Virtual memory working (page tables, isolation)
3. ✅ Preemptive scheduling via timer interrupts
4. ✅ Interrupt handling (timer, keyboard via serial)
5. ✅ VirtIO block device working (read/write)
6. ✅ CommitLog persists to disk
7. ✅ System can replay from disk CommitLog after reboot
8. ✅ All Phase 1 tests pass on QEMU
9. ✅ Core invariants verified
10. ✅ Performance: can handle 10,000 syscalls/second

## Migration from Phase 1

### Code Reuse

Most Phase 1 code is platform-independent:
- `Zero-axiom` - No changes needed
- `Zero-kernel` core logic - Minimal changes
- Process library - Update for native syscall ABI
- Init/services - Recompile for native target

### HAL Implementation

Create `x86_64.rs` HAL:
- `spawn_process()` - Create page tables, load ELF
- `allocate()` - Physical memory allocator
- `now_nanos()` - Read HPET/TSC
- `debug_write()` - Serial port output

## Related Documentation

- [Phase 1: WASM](../phase-1-wasm/README.md) - Foundation
- [Spec: HAL](../../spec/01-hal/01-targets.md) - QEMU target
- [Spec: Kernel VMM](../../spec/03-kernel/02-vmm.md) - Virtual memory
- [Spec: Interrupts](../../spec/03-kernel/05-interrupts.md) - IRQ handling

## Next Phase

After Phase 2 is complete, proceed to [Phase 3: Bare Metal](../phase-3-baremetal/README.md) for real hardware.
