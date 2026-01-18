# Phase 3: Bare Metal (Real Hardware)

> **Goal**: Port Orbital OS to real hardware (x86_64, ARM64).

## Overview

Phase 3 transitions from QEMU to real hardware, adding:
- Real device drivers (NVMe, SATA, USB, network cards)
- UEFI boot support
- Multi-core support (SMP)
- Advanced power management (ACPI)
- Hardware security (TPM, secure boot)

This phase proves the kernel can manage production hardware.

### Platform Transition

| Feature              | Phase 2 (QEMU)     | Phase 3 (Bare Metal) |
|----------------------|--------------------|----------------------|
| **Boot**             | Simple bootloader  | UEFI                 |
| **Storage**          | virtio-blk         | NVMe/SATA/AHCI       |
| **Network**          | virtio-net         | Intel e1000, Realtek |
| **Entropy**          | virtio-rng         | RDRAND, TPM          |
| **Timer**            | PIT/HPET           | TSC, HPET            |
| **Multi-core**       | Single CPU         | SMP (multiple CPUs)  |
| **Firmware**         | BIOS/Legacy        | UEFI                 |

## Implementation Stages

| Stage | Name | Focus | Test Hardware |
|-------|------|-------|---------------|
| [3.1](stage-3.1-uefi-boot.md) | UEFI Boot | Boot from UEFI, GOP framebuffer | Modern PC, VM |
| [3.2](stage-3.2-nvme-driver.md) | NVMe Driver | Real storage driver | PC with NVMe |
| [3.3](stage-3.3-network-driver.md) | Network Driver | Intel e1000 or Realtek | PC with NIC |
| [3.4](stage-3.4-smp-multicore.md) | SMP Multi-core | Multiple CPUs, per-CPU data | Multi-core PC |
| [3.5](stage-3.5-acpi-power.md) | ACPI + Power | Power management, shutdown/reboot | Real PC |
| [3.6](stage-3.6-security.md) | Security Features | TPM, secure boot, measured boot | TPM-enabled PC |
| [3.7](stage-3.7-production.md) | Production Ready | Stability, performance, deployment | Various hardware |

## Core Invariants (Still True)

All previous invariants must still hold:

- ✅ Two-log model (SysLog + CommitLog)
- ✅ Capability integrity
- ✅ Deterministic replay
- ✅ Sender verification
- ✅ Memory isolation (hardware-enforced)

## New Challenges

### 1. Hardware Diversity

- Different CPUs (Intel, AMD, ARM)
- Different device configurations
- Driver compatibility matrix
- Firmware variations (UEFI implementations differ)

### 2. Multi-Core Synchronization

- Per-CPU kernel stacks
- Spinlocks for shared data
- IPI (inter-processor interrupts)
- Load balancing across cores

### 3. Real Device Drivers

- DMA (Direct Memory Access)
- MMIO (Memory-Mapped I/O)
- MSI/MSI-X interrupts
- Device-specific quirks

### 4. Performance

- Optimize syscall path (<100 ns)
- Minimize context switch overhead
- Efficient IPC (zero-copy where possible)
- Lock-free data structures

### 5. Security

- Secure boot chain (UEFI → bootloader → kernel)
- TPM for key storage
- Measured boot (record boot chain in TPM)
- IOMMU for DMA protection

## Success Criteria for Phase 3

Phase 3 is complete when:

1. ✅ Boots on real hardware via UEFI
2. ✅ NVMe storage working (read/write)
3. ✅ Network connectivity working (TCP/IP stack)
4. ✅ Multi-core support (4+ cores)
5. ✅ Power management (shutdown, reboot, suspend)
6. ✅ Secure boot chain verified
7. ✅ TPM integration for key storage
8. ✅ Performance: 1M syscalls/second per core
9. ✅ Stability: 24-hour stress test passes
10. ✅ All invariants verified on hardware

## Target Hardware

### Minimum Requirements

- **CPU**: x86_64 with UEFI, or ARM64
- **RAM**: 512 MB minimum, 2 GB recommended
- **Storage**: NVMe or SATA SSD (100 MB minimum)
- **Network**: Any common NIC (Intel e1000, Realtek)
- **Firmware**: UEFI 2.x

### Tested Hardware

- **Intel NUC** (various models)
- **Raspberry Pi 4** (ARM64)
- **ThinkPad** laptops (x86_64)
- **Custom server** hardware
- **Cloud VMs** (AWS, GCP, Azure)

## Development Workflow

### Building for Bare Metal

```bash
# Build kernel
cargo build --release --target x86_64-unknown-none

# Create UEFI image
make uefi-image

# Flash to USB drive
sudo dd if=target/uefi.img of=/dev/sdX bs=1M
```

### Testing

1. **VM Testing**: Test on QEMU with UEFI firmware first
2. **USB Boot**: Flash to USB, boot on test hardware
3. **Serial Console**: Use USB-serial adapter for debug output
4. **Network Boot**: PXE boot for automated testing

### Debugging

- Serial console (115200 baud)
- JTAG debugger (for deep hardware issues)
- Kernel logs to network syslog
- Crash dumps to NVMe

## File Structure

```
crates/
  orbital-hal/
    src/
      x86_64/
        uefi.rs
        nvme.rs
        e1000.rs
        smp.rs
        acpi.rs
  orbital-boot/
    src/
      uefi_boot.rs

  # New crates for hardware
  orbital-drivers/
    src/
      nvme.rs
      ahci.rs
      e1000.rs
      rtl8139.rs

  orbital-security/
    src/
      tpm.rs
      secure_boot.rs
```

## Related Documentation

- [Phase 2: QEMU](../phase-2-qemu/README.md) - Virtual hardware
- [Spec: HAL](../../spec/01-hal/01-targets.md) - Bare metal target
- UEFI Specification
- NVMe Specification
- Intel/AMD Developer Manuals

## Deployment

After Phase 3, Orbital OS can be deployed to:

- **Edge devices**: IoT, embedded systems
- **Servers**: Data center infrastructure
- **Desktops**: Workstations with GUI (Phase 4)
- **Cloud**: VM images for cloud providers

Next: Phase 4 (Visual OS) adds desktop environment and compositor.
