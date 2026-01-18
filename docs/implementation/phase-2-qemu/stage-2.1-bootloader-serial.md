# Stage 2.1: Bootloader + Serial

> **Goal**: Boot kernel in QEMU and print to serial console.

## Tasks

1. Create bootloader entry point
2. Set up GDT (Global Descriptor Table)
3. Set up IDT (Interrupt Descriptor Table) - minimal
4. Initialize serial port (COM1)
5. Print "Hello from QEMU kernel!"

## Test

```bash
make qemu
```

Expected output:
```
Hello from QEMU kernel!
```

## Key Files

- `crates/orbital-boot/src/boot.s` - Assembly entry
- `crates/orbital-boot/src/lib.rs` - Early init
- `crates/orbital-hal/src/x86_64/serial.rs` - Serial driver

## Next

[Stage 2.2: VMM + Paging](stage-2.2-vmm-paging.md)
