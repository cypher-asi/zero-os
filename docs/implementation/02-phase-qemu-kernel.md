# Phase 2: QEMU Kernel

**Duration:** 8-12 weeks  
**Status:** Implementation Phase  
**Prerequisites:** Phase 1 (Hosted Simulator)

---

## Objective

Build a minimal microkernel that boots in QEMU, provides memory management, SMP scheduling, capability-based security, and IPC primitives.

---

## Deliverables

### 2.1 Boot Infrastructure

| Component | Description | Complexity |
|-----------|-------------|------------|
| Multiboot2 header | Boot protocol compliance | Low |
| Early init | Stack setup, BSS clearing | Low |
| Serial output | Debug console | Low |
| GDT/IDT setup | x86_64 tables | Medium |
| Page table init | Initial paging | Medium |

### 2.2 Memory Management

| Component | Description | Complexity |
|-----------|-------------|------------|
| Physical allocator | Frame allocation | Medium |
| Virtual memory | Page table management | High |
| Address spaces | Per-process page tables | High |
| Kernel heap | Kernel allocator | Medium |
| User mapping | User-space memory | Medium |

### 2.3 Scheduler

| Component | Description | Complexity |
|-----------|-------------|------------|
| Thread structure | Thread control block | Medium |
| Context switch | Register save/restore | High |
| Run queues | Priority queues | Medium |
| Timer interrupt | Preemption | Medium |
| SMP support | Per-CPU state | High |

### 2.4 Capability System

| Component | Description | Complexity |
|-----------|-------------|------------|
| Capability table | Per-process cap space | Medium |
| Token validation | Check authenticity | Medium |
| Grant/revoke | Capability delegation | Medium |
| Lookup | Fast cap lookup | Low |

### 2.5 IPC

| Component | Description | Complexity |
|-----------|-------------|------------|
| Endpoints | IPC communication points | Medium |
| Send/Receive | Basic message passing | High |
| Call/Reply | RPC pattern | High |
| Cap transfer | Pass caps in messages | Medium |

### 2.6 Init Process

| Component | Description | Complexity |
|-----------|-------------|------------|
| ELF loader | Load user binaries | Medium |
| Init binary | First user process | Low |
| System caps | Grant initial caps | Low |

---

## Technical Approach

### Boot Sequence

```rust
// Entry point from bootloader
#[no_mangle]
pub extern "C" fn kernel_main(boot_info: &BootInfo) -> ! {
    // Initialize serial for debugging
    serial::init();
    kprintln!("Orbital kernel starting...");
    
    // Setup GDT and IDT
    gdt::init();
    idt::init();
    
    // Initialize memory
    memory::init(boot_info);
    
    // Initialize per-CPU state
    percpu::init();
    
    // Initialize scheduler
    scheduler::init();
    
    // Initialize IPC
    ipc::init();
    
    // Create init process
    let init = process::create_init();
    scheduler::add_thread(init.main_thread());
    
    // Start scheduling
    scheduler::start();
}
```

### Context Switch

```rust
// Context switch implementation (x86_64)
#[naked]
pub unsafe extern "C" fn context_switch(
    old_ctx: *mut CpuContext,
    new_ctx: *const CpuContext,
) {
    asm!(
        // Save old context
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov [rdi], rsp",      // Save old stack pointer
        
        // Load new context
        "mov rsp, [rsi]",      // Load new stack pointer
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "ret",
        options(noreturn)
    )
}
```

### IPC Implementation

```rust
pub fn sys_call(
    endpoint: CapSlot,
    message: &Message,
) -> Result<Message, IpcError> {
    let current = current_thread();
    
    // Validate capability
    let cap = current.process().lookup_cap(endpoint)?;
    if !cap.permissions.write {
        return Err(IpcError::PermissionDenied);
    }
    
    let ep = endpoints().get(cap.object_id)?;
    
    // Fast path: server is waiting
    if let Some(server) = ep.waiting_receiver() {
        // Direct transfer
        transfer_message(current, server, message)?;
        
        // Block waiting for reply
        current.block(BlockReason::WaitingReply { from: server.id() });
        
        // Wake server
        scheduler::ready(server);
        scheduler::schedule();
        
        // Return reply when woken
        Ok(current.reply_message().take().unwrap())
    } else {
        // Slow path: queue message
        ep.queue_message(message.clone(), current.id());
        current.block(BlockReason::WaitingReply { from: ThreadId::UNKNOWN });
        scheduler::schedule();
        Ok(current.reply_message().take().unwrap())
    }
}
```

---

## Implementation Steps

### Week 1-2: Boot & Early Init

1. Create cargo project with custom target
2. Implement Multiboot2 header
3. Setup GDT and IDT
4. Initialize serial output
5. Create initial page tables
6. Test boot in QEMU

### Week 3-4: Memory Management

1. Implement physical frame allocator
2. Create page table management
3. Implement kernel heap
4. Create address space abstraction
5. Implement user-space mapping
6. Add memory protection

### Week 5-6: Scheduler

1. Define thread structure
2. Implement context switch
3. Create run queues
4. Setup timer interrupt
5. Implement preemption
6. Add SMP support

### Week 7-8: Capabilities & IPC

1. Define capability structure
2. Implement capability table
3. Create endpoint objects
4. Implement send/receive
5. Implement call/reply
6. Add capability transfer

### Week 9-10: Init Process

1. Implement ELF loader
2. Create init binary
3. Grant system capabilities
4. Boot user space
5. Test IPC between processes

### Week 11-12: Integration & Testing

1. End-to-end testing
2. Multi-process testing
3. SMP testing
4. Bug fixes
5. Documentation

---

## Test Strategy

### Unit Tests (where possible)

```rust
#[test]
fn frame_allocator_works() {
    let mut allocator = FrameAllocator::new(test_regions());
    
    let frame1 = allocator.allocate().unwrap();
    let frame2 = allocator.allocate().unwrap();
    
    assert_ne!(frame1, frame2);
    
    allocator.free(frame1);
    let frame3 = allocator.allocate().unwrap();
    
    assert_eq!(frame1, frame3); // Reused
}
```

### QEMU Integration Tests

```rust
#[test_case]
fn basic_ipc() {
    // Create two processes
    let server = create_test_process("echo_server");
    let client = create_test_process("echo_client");
    
    // Run until completion
    run_until_idle();
    
    // Check results
    assert_eq!(client.exit_code(), 0);
}
```

---

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| Kernel boots | QEMU test |
| Memory isolation works | Protection fault tests |
| Preemption works | Multi-thread tests |
| SMP works | Multi-CPU QEMU tests |
| IPC works | Client-server tests |
| Capabilities enforced | Permission tests |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Context switch bugs | Extensive testing, reference other kernels |
| SMP race conditions | Lock discipline, careful design |
| Page table bugs | Incremental testing |
| IPC deadlocks | Timeout mechanisms |

---

## Exit Criteria

Phase 2 is complete when:

- [ ] Kernel boots in QEMU
- [ ] Multiple processes run
- [ ] IPC works correctly
- [ ] SMP works with multiple CPUs
- [ ] Capabilities enforced
- [ ] Documentation complete

---

*[← Phase 1](01-phase-hosted-simulator.md) | [Phase 3: Storage →](03-phase-storage-filesystem.md)*
