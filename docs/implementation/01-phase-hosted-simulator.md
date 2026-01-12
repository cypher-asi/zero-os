# Phase 1: Hosted Simulator

**Duration:** 8-12 weeks  
**Status:** Implementation Phase  
**Prerequisites:** None (starting point)

---

## Objective

Build a fully functional Orbital OS simulator running as a Rust binary on an existing host OS. This phase proves the Axiom architecture and three-phase action model before tackling kernel development.

---

## Deliverables

### 1.1 Axiom Core

| Component | Description | Complexity |
|-----------|-------------|------------|
| Entry format | Serializable entry structure | Low |
| Hash chain | BLAKE3-based chaining | Low |
| Storage backend | File-based append-only log | Medium |
| WAL | Write-ahead logging for crash safety | Medium |
| Recovery | Crash recovery from WAL | Medium |

### 1.2 Sequencer

| Component | Description | Complexity |
|-----------|-------------|------------|
| Proposal handling | Accept/reject proposals | Low |
| Validation | Check proposal validity | Medium |
| Ordering | Assign sequence numbers | Low |
| Batching | Group proposals for efficiency | Medium |
| Notification | Notify subscribers | Low |

### 1.3 Reducer

| Component | Description | Complexity |
|-----------|-------------|------------|
| State structure | Control state definition | Medium |
| Reduction logic | Entry → state transformation | Medium |
| Incremental update | Apply single entry | Low |
| Full replay | Rebuild from genesis | Low |
| Verification | Check reduction correctness | Medium |

### 1.4 Service Framework

| Component | Description | Complexity |
|-----------|-------------|------------|
| Service trait | Common service interface | Low |
| IPC simulation | Channel-based message passing | Medium |
| Service registry | Discover services | Low |
| Lifecycle management | Start/stop services | Medium |
| Health checking | Monitor service health | Low |

### 1.5 Terminal Service

| Component | Description | Complexity |
|-----------|-------------|------------|
| Input handling | Stdin processing | Low |
| Output formatting | Stdout formatting | Low |
| Command parser | Parse commands | Medium |
| Built-in commands | cd, ls, help, etc. | Low |
| Axiom commands | Inspect Axiom | Medium |
| Job commands | Submit/monitor jobs | Medium |

### 1.6 Job Executor

| Component | Description | Complexity |
|-----------|-------------|------------|
| Manifest parsing | Parse job manifests | Medium |
| Input fetching | Content-addressed inputs | Medium |
| Environment setup | Sandboxed execution | High |
| Execution | Run deterministic jobs | High |
| Output capture | Collect outputs | Medium |
| Receipt generation | Create verification receipts | Medium |

---

## Technical Approach

### Axiom Implementation

```rust
// Core Axiom structure
pub struct HostedAxiom {
    /// Storage path
    path: PathBuf,
    
    /// Current state
    state: AxiomState,
    
    /// WAL for crash recovery
    wal: WriteAheadLog,
    
    /// Subscribers
    subscribers: Vec<Box<dyn AxiomSubscriber>>,
}

impl HostedAxiom {
    pub fn open(path: &Path) -> Result<Self, AxiomError> {
        let state = Self::recover_or_create(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            state,
            wal: WriteAheadLog::open(path.join("wal"))?,
            subscribers: vec![],
        })
    }
    
    pub fn append(&mut self, entry: AxiomEntry) -> Result<u64, AxiomError> {
        // Write to WAL first
        self.wal.begin_append(entry.header.sequence)?;
        
        // Append to main log
        self.state.append(&entry)?;
        
        // Commit WAL
        self.wal.commit_append(entry.header.sequence)?;
        
        // Notify subscribers
        for sub in &mut self.subscribers {
            sub.on_entry(entry.header.sequence, &entry);
        }
        
        Ok(entry.header.sequence)
    }
}
```

### Service Framework

```rust
// Simulated IPC using channels
pub struct HostedIpc {
    channels: HashMap<EndpointId, (Sender<Message>, Receiver<Message>)>,
}

impl HostedIpc {
    pub fn create_endpoint(&mut self) -> EndpointId {
        let (tx, rx) = channel();
        let id = EndpointId::new();
        self.channels.insert(id, (tx, rx));
        id
    }
    
    pub fn send(&self, endpoint: EndpointId, msg: Message) -> Result<(), IpcError> {
        let (tx, _) = self.channels.get(&endpoint)
            .ok_or(IpcError::InvalidEndpoint)?;
        tx.send(msg).map_err(|_| IpcError::Disconnected)
    }
    
    pub fn receive(&self, endpoint: EndpointId) -> Result<Message, IpcError> {
        let (_, rx) = self.channels.get(&endpoint)
            .ok_or(IpcError::InvalidEndpoint)?;
        rx.recv().map_err(|_| IpcError::Disconnected)
    }
}
```

---

## Implementation Steps

### Week 1-2: Axiom Core

1. Define entry format and serialization
2. Implement hash computation
3. Create file-based storage
4. Add WAL support
5. Implement crash recovery
6. Write property-based tests

### Week 3-4: Sequencer & Reducer

1. Implement proposal submission
2. Add validation logic
3. Implement ordering
4. Create reducer framework
5. Implement state derivation
6. Add replay verification

### Week 5-6: Service Framework

1. Define service interface
2. Implement channel-based IPC
3. Create service registry
4. Add lifecycle management
5. Implement supervisor
6. Add health checking

### Week 7-8: Terminal & Jobs

1. Implement terminal input/output
2. Create command parser
3. Add built-in commands
4. Implement job manifest parsing
5. Create job executor
6. Add receipt generation

### Week 9-10: Integration & Testing

1. Integrate all components
2. End-to-end testing
3. Crash recovery testing
4. Performance profiling
5. Documentation

### Week 11-12: Polish & Hardening

1. Fix bugs from testing
2. Performance optimization
3. Code cleanup
4. Final documentation

---

## Test Strategy

### Unit Tests

```rust
#[test]
fn axiom_entries_chain_correctly() {
    let mut axiom = HostedAxiom::new_temp();
    
    let entry1 = axiom.append_test_entry("first").unwrap();
    let entry2 = axiom.append_test_entry("second").unwrap();
    
    assert_eq!(entry2.header.prev_hash, entry1.compute_hash());
}

#[test]
fn reducer_is_deterministic() {
    let axiom = create_test_axiom(100);
    
    let state1 = Reducer::reduce(&axiom);
    let state2 = Reducer::reduce(&axiom);
    
    assert_eq!(state1, state2);
}

#[test]
fn crash_recovery_preserves_integrity() {
    let path = temp_dir();
    
    // Simulate crash after partial write
    {
        let mut axiom = HostedAxiom::open(&path).unwrap();
        axiom.simulate_crash_during_append();
    }
    
    // Recover
    let axiom = HostedAxiom::open(&path).unwrap();
    assert!(axiom.verify_chain().is_ok());
}
```

### Integration Tests

```rust
#[test]
fn three_phase_action_completes() {
    let system = TestSystem::new();
    
    // Phase 1: Create proposal
    let proposal = system.create_file_proposal("/test.txt", "content");
    
    // Phase 2: Submit and commit
    let result = system.sequencer.submit(proposal);
    assert!(matches!(result, CommitResult::Committed { .. }));
    
    // Phase 3: Verify effects materialized
    let content = system.fs.read("/test.txt").unwrap();
    assert_eq!(content, "content");
    
    // Verify receipt exists
    let receipt = system.receipts.get_for_entry(result.sequence()).unwrap();
    assert!(receipt.verify().is_ok());
}
```

---

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| Axiom chains correctly | Property-based tests |
| Reducer is deterministic | Replay tests |
| Crash recovery works | Fault injection tests |
| Three-phase actions complete | Integration tests |
| Jobs execute deterministically | Replay verification |
| Terminal is functional | Manual testing |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| WAL complexity | Use existing WAL library as reference |
| IPC performance | Profile early, optimize hot paths |
| Determinism bugs | Extensive property-based testing |
| Serialization issues | Use well-tested serde formats |

---

## Exit Criteria

Phase 1 is complete when:

- [ ] All components implemented and tested
- [ ] Property-based tests pass
- [ ] Integration tests pass
- [ ] Crash recovery verified
- [ ] Documentation complete
- [ ] Code reviewed

---

*[← Roadmap](00-roadmap.md) | [Phase 2: QEMU Kernel →](02-phase-qemu-kernel.md)*
