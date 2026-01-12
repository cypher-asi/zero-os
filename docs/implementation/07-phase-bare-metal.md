# Phase 7: Bare Metal

**Duration:** 8-12 weeks  
**Status:** Implementation Phase  
**Prerequisites:** Phase 6 (Transactional Upgrades)

---

## Objective

Port Orbital OS to boot and run on real x86_64 hardware with production-quality drivers.

---

## Deliverables

### 7.1 UEFI Bootloader

| Component | Description | Complexity |
|-----------|-------------|------------|
| UEFI application | Boot from UEFI | Medium |
| Memory map | Get memory layout | Low |
| Graphics setup | Framebuffer init | Medium |
| Kernel loading | Load kernel image | Medium |

### 7.2 Hardware Drivers

| Component | Description | Complexity |
|-----------|-------------|------------|
| AHCI driver | SATA storage | High |
| NVMe driver | NVMe storage | High |
| USB stack | USB support | Very High |
| Intel NIC | e1000/igb driver | High |
| Realtek NIC | r8169 driver | Medium |

### 7.3 Platform Support

| Component | Description | Complexity |
|-----------|-------------|------------|
| ACPI | Power management | High |
| HPET/RTC | Timers | Medium |
| IOAPIC | Interrupt routing | Medium |
| PCI enumeration | Device discovery | Medium |

### 7.4 Console

| Component | Description | Complexity |
|-----------|-------------|------------|
| PS/2 keyboard | Keyboard input | Low |
| VGA text mode | Text output | Low |
| Framebuffer | Graphics output | Medium |

---

## Technical Approach

### UEFI Bootloader

```rust
// UEFI bootloader entry point
#[entry]
fn efi_main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // Initialize UEFI services
    uefi_services::init(&mut system_table).unwrap();
    info!("Orbital UEFI bootloader starting...");
    
    // Get memory map
    let mmap_size = system_table.boot_services().memory_map_size();
    let mut mmap_buf = vec![0u8; mmap_size.map_size + 2 * mmap_size.entry_size];
    
    // Load kernel
    let kernel = load_kernel(&system_table)?;
    info!("Kernel loaded at {:?}", kernel.entry_point);
    
    // Setup framebuffer
    let framebuffer = setup_framebuffer(&system_table)?;
    
    // Exit boot services
    let (runtime, mmap) = system_table
        .exit_boot_services(handle, &mut mmap_buf)
        .unwrap();
    
    // Jump to kernel
    let boot_info = BootInfo {
        memory_map: mmap,
        framebuffer,
        acpi_rsdp: find_acpi_rsdp(&runtime),
    };
    
    unsafe {
        let entry: extern "C" fn(&BootInfo) -> ! = 
            core::mem::transmute(kernel.entry_point);
        entry(&boot_info);
    }
}
```

### AHCI Driver

```rust
pub struct AhciController {
    /// MMIO base address
    base: *mut AhciRegisters,
    
    /// Ports
    ports: [Option<AhciPort>; 32],
    
    /// Command list
    command_list: DmaBuffer<CommandHeader>,
    
    /// FIS receive buffer
    fis_buffer: DmaBuffer<ReceivedFis>,
}

impl AhciController {
    pub fn init(pci_device: &PciDevice) -> Result<Self, AhciError> {
        // Map MMIO registers
        let bar5 = pci_device.bar(5)?;
        let base = map_mmio(bar5.address, bar5.size)?;
        
        // Read capabilities
        let cap = unsafe { (*base).cap };
        let num_ports = (cap & 0x1F) + 1;
        
        // Initialize each port
        let mut ports = [None; 32];
        for i in 0..num_ports {
            if let Some(port) = AhciPort::init(base, i)? {
                ports[i as usize] = Some(port);
            }
        }
        
        Ok(Self {
            base,
            ports,
            command_list: DmaBuffer::new(32)?,
            fis_buffer: DmaBuffer::new(1)?,
        })
    }
    
    pub async fn read(
        &mut self,
        port: usize,
        lba: u64,
        sectors: u16,
        buffer: &mut [u8],
    ) -> Result<(), AhciError> {
        let port = self.ports[port].as_mut()
            .ok_or(AhciError::PortNotPresent)?;
        
        // Build command
        let cmd = SataCommand::ReadDma {
            lba,
            sector_count: sectors,
        };
        
        // Issue command
        port.issue_command(&cmd, buffer).await
    }
}
```

### PCI Enumeration

```rust
pub struct PciScanner;

impl PciScanner {
    pub fn scan() -> Vec<PciDevice> {
        let mut devices = Vec::new();
        
        for bus in 0..256u16 {
            for device in 0..32u8 {
                for function in 0..8u8 {
                    if let Some(dev) = Self::probe(bus as u8, device, function) {
                        devices.push(dev);
                    }
                }
            }
        }
        
        devices
    }
    
    fn probe(bus: u8, device: u8, function: u8) -> Option<PciDevice> {
        let vendor_id = pci_read_config_u16(bus, device, function, 0x00);
        
        if vendor_id == 0xFFFF {
            return None;
        }
        
        let device_id = pci_read_config_u16(bus, device, function, 0x02);
        let class = pci_read_config_u8(bus, device, function, 0x0B);
        let subclass = pci_read_config_u8(bus, device, function, 0x0A);
        
        Some(PciDevice {
            bus,
            device,
            function,
            vendor_id,
            device_id,
            class,
            subclass,
        })
    }
}
```

---

## Implementation Steps

### Week 1-2: UEFI Bootloader

1. Create UEFI application
2. Implement memory map handling
3. Add kernel loading
4. Setup framebuffer
5. Pass boot info to kernel

### Week 3-4: Platform Support

1. Implement ACPI parsing
2. Setup IOAPIC
3. Configure timers (HPET/RTC)
4. PCI enumeration

### Week 5-6: Storage Drivers

1. Implement AHCI driver
2. Add NVMe driver
3. Test on real hardware

### Week 7-8: Network Drivers

1. Implement Intel e1000 driver
2. Add Realtek r8169 driver
3. Test networking

### Week 9-10: Console & USB

1. Implement PS/2 keyboard
2. Add framebuffer console
3. Basic USB stack (optional)

### Week 11-12: Integration & Testing

1. Full system testing on hardware
2. Stability testing
3. Performance optimization
4. Documentation

---

## Hardware Test Matrix

| Component | Test Hardware |
|-----------|--------------|
| UEFI boot | Various UEFI systems |
| AHCI | SATA SSDs and HDDs |
| NVMe | NVMe SSDs |
| Network | Intel I210, Realtek RTL8111 |
| USB | Optional for this phase |

---

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| System boots | Real hardware test |
| Storage works | Read/write test |
| Network works | Ping test |
| System stable | Extended run test |

---

*[← Phase 6](06-phase-transactional-upgrades.md) | [Phase 8: Visual OS →](08-phase-visual-os.md)*
