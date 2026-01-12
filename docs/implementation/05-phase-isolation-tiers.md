# Phase 5: Isolation Tiers

**Duration:** 4-6 weeks  
**Status:** Implementation Phase  
**Prerequisites:** Phase 4 (Networking)

---

## Objective

Implement resource limits, process sandboxing, and fine-grained capability scoping to provide robust isolation between processes.

---

## Deliverables

### 5.1 Resource Limits

| Component | Description | Complexity |
|-----------|-------------|------------|
| CPU limits | Time-based CPU quota | Medium |
| Memory limits | Resident memory cap | Medium |
| I/O limits | Bandwidth throttling | Medium |
| Thread limits | Max threads per process | Low |

### 5.2 Process Sandboxing

| Component | Description | Complexity |
|-----------|-------------|------------|
| Syscall filtering | Block dangerous syscalls | Medium |
| Namespace isolation | Filesystem view | High |
| Network isolation | Network namespace | Medium |

### 5.3 Capability Scoping

| Component | Description | Complexity |
|-----------|-------------|------------|
| Capability attenuation | Reduce permissions | Low |
| Capability expiry | Time-limited caps | Medium |
| Capability delegation | Track delegation chain | Medium |

### 5.4 Quota Management

| Component | Description | Complexity |
|-----------|-------------|------------|
| Usage tracking | Monitor resource use | Medium |
| Quota enforcement | Block excess usage | Medium |
| Quota reporting | Usage statistics | Low |

---

## Technical Approach

### Resource Limits

```rust
#[derive(Clone, Debug)]
pub struct ResourceLimits {
    /// CPU time limit (nanoseconds per second)
    pub cpu_quota: u64,
    
    /// Maximum resident memory (bytes)
    pub memory_limit: usize,
    
    /// I/O bandwidth limit (bytes per second)
    pub io_bandwidth: u64,
    
    /// Maximum threads
    pub max_threads: u32,
    
    /// Maximum capabilities
    pub max_capabilities: u32,
}

pub struct ResourceEnforcer {
    limits: ResourceLimits,
    usage: ResourceUsage,
    period_start: Timestamp,
}

impl ResourceEnforcer {
    pub fn check_cpu(&mut self, elapsed: u64) -> Result<(), LimitError> {
        self.usage.cpu_time += elapsed;
        
        // Check against quota for current period
        let period_elapsed = Timestamp::now() - self.period_start;
        if period_elapsed.as_nanos() >= 1_000_000_000 {
            // Reset for new period
            self.period_start = Timestamp::now();
            self.usage.cpu_time = 0;
        }
        
        if self.usage.cpu_time > self.limits.cpu_quota {
            return Err(LimitError::CpuExceeded);
        }
        
        Ok(())
    }
    
    pub fn check_memory(&self, request: usize) -> Result<(), LimitError> {
        if self.usage.memory + request > self.limits.memory_limit {
            return Err(LimitError::MemoryExceeded);
        }
        Ok(())
    }
}
```

### Syscall Filtering

```rust
pub struct SyscallFilter {
    /// Allowed syscalls
    allowed: HashSet<u64>,
    
    /// Default action for unlisted syscalls
    default_action: FilterAction,
}

impl SyscallFilter {
    pub fn check(&self, syscall: u64) -> FilterAction {
        if self.allowed.contains(&syscall) {
            FilterAction::Allow
        } else {
            self.default_action
        }
    }
    
    /// Create filter for deterministic jobs
    pub fn deterministic() -> Self {
        let mut allowed = HashSet::new();
        
        // Memory management
        allowed.insert(SYS_MMAP);
        allowed.insert(SYS_MUNMAP);
        allowed.insert(SYS_MPROTECT);
        
        // File I/O (sandboxed paths only)
        allowed.insert(SYS_READ);
        allowed.insert(SYS_WRITE);
        allowed.insert(SYS_OPEN);
        allowed.insert(SYS_CLOSE);
        
        // Process exit
        allowed.insert(SYS_EXIT);
        allowed.insert(SYS_EXIT_GROUP);
        
        Self {
            allowed,
            default_action: FilterAction::Kill,
        }
    }
}
```

### Capability Attenuation

```rust
impl Capability {
    /// Create attenuated capability with reduced permissions
    pub fn attenuate(&self, new_perms: Permissions) -> Result<Capability, CapError> {
        // New permissions must be subset of existing
        if !self.permissions.contains(new_perms) {
            return Err(CapError::CannotAmplify);
        }
        
        Ok(Capability {
            id: CapabilityId::new(),
            object_type: self.object_type,
            object_id: self.object_id,
            permissions: new_perms,
            generation: self.generation,
            parent: Some(self.id),
        })
    }
    
    /// Create time-limited capability
    pub fn with_expiry(&self, expires_at: Timestamp) -> Capability {
        Capability {
            expiry: Some(expires_at),
            ..self.clone()
        }
    }
}
```

---

## Implementation Steps

### Week 1-2: Resource Limits

1. Implement CPU quota tracking
2. Add memory limit enforcement
3. Implement I/O throttling
4. Add thread limits
5. Create enforcement hooks

### Week 3-4: Sandboxing

1. Implement syscall filtering
2. Add namespace isolation
3. Create sandbox profiles
4. Integrate with job executor

### Week 5-6: Capabilities & Testing

1. Implement capability attenuation
2. Add capability expiry
3. Track delegation chains
4. Integration testing
5. Security testing

---

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| CPU limits enforced | Quota exhaustion test |
| Memory limits work | OOM handling test |
| Syscall filtering works | Blocked syscall test |
| Capability attenuation works | Permission test |

---

*[← Phase 4](04-phase-networking.md) | [Phase 6: Upgrades →](06-phase-transactional-upgrades.md)*
