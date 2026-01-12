# Phase 6: Transactional Upgrades

**Duration:** 4-6 weeks  
**Status:** Implementation Phase  
**Prerequisites:** Phase 5 (Isolation Tiers)

---

## Objective

Implement atomic system image updates with automatic rollback on failure, ensuring the system can always boot to a known-good state.

---

## Deliverables

### 6.1 Image Format

| Component | Description | Complexity |
|-----------|-------------|------------|
| Image structure | Content-addressed layout | Medium |
| Image signing | Cryptographic signatures | Medium |
| Image verification | Hash verification | Low |
| Manifest format | Component listing | Low |

### 6.2 Image Manager

| Component | Description | Complexity |
|-----------|-------------|------------|
| Image staging | Download/stage new image | Medium |
| Image verification | Verify before activation | Medium |
| Image activation | Switch active image | High |
| Image cleanup | Remove old images | Low |

### 6.3 Upgrade Service

| Component | Description | Complexity |
|-----------|-------------|------------|
| Upgrade orchestration | Coordinate upgrade | High |
| Health checking | Post-upgrade validation | Medium |
| Rollback trigger | Detect failures | Medium |

### 6.4 Bootloader Integration

| Component | Description | Complexity |
|-----------|-------------|------------|
| Boot selection | Choose active image | Medium |
| Rollback boot | Boot previous on failure | Medium |
| Boot counter | Track boot attempts | Low |

---

## Technical Approach

### Image Structure

```rust
/// System image manifest
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageManifest {
    /// Manifest version
    pub version: u32,
    
    /// Image identifier
    pub id: ImageId,
    
    /// Image version
    pub image_version: Version,
    
    /// Root hash of image content
    pub root_hash: Hash,
    
    /// Components
    pub components: Vec<ComponentRef>,
    
    /// Signature
    pub signature: Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentRef {
    /// Component name
    pub name: String,
    
    /// Component type
    pub component_type: ComponentType,
    
    /// Content hash
    pub hash: Hash,
    
    /// Size in bytes
    pub size: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComponentType {
    Kernel,
    InitRamfs,
    Service,
    Driver,
    Config,
}
```

### Upgrade Process

```rust
pub struct UpgradeService {
    image_manager: ImageManager,
    axiom: AxiomClient,
    bootloader: BootloaderClient,
}

impl UpgradeService {
    pub async fn upgrade(&mut self, new_image: ImageId) -> Result<(), UpgradeError> {
        // Stage 1: Verify new image
        let manifest = self.image_manager.verify(&new_image).await?;
        
        // Stage 2: Commit upgrade intent to Axiom
        let proposal = Proposal {
            entry_type: EntryType::ImageStage,
            payload: ImageStagePayload {
                image_id: new_image,
                manifest_hash: manifest.root_hash,
            }.into(),
            ..Default::default()
        };
        
        let staging_entry = self.axiom.submit(proposal).await?;
        
        // Stage 3: Prepare bootloader
        self.bootloader.stage_image(&manifest).await?;
        
        // Stage 4: Commit activation
        let proposal = Proposal {
            entry_type: EntryType::ImageActivate,
            payload: ImageActivatePayload {
                image_id: new_image,
                staging_entry: staging_entry.sequence(),
            }.into(),
            ..Default::default()
        };
        
        self.axiom.submit(proposal).await?;
        
        // Stage 5: Reboot into new image
        self.bootloader.set_next_boot(&new_image)?;
        self.bootloader.reboot()?;
        
        Ok(())
    }
}
```

### Rollback Mechanism

```rust
pub struct BootManager {
    current_image: ImageId,
    previous_image: Option<ImageId>,
    boot_counter: u32,
    max_boot_attempts: u32,
}

impl BootManager {
    pub fn on_boot(&mut self) -> BootDecision {
        self.boot_counter += 1;
        
        if self.boot_counter > self.max_boot_attempts {
            // Too many failed boots - rollback
            if let Some(prev) = &self.previous_image {
                return BootDecision::Rollback(prev.clone());
            }
        }
        
        BootDecision::Continue
    }
    
    pub fn mark_boot_successful(&mut self) {
        self.boot_counter = 0;
    }
    
    pub fn rollback(&mut self) -> Result<(), RollbackError> {
        let previous = self.previous_image.take()
            .ok_or(RollbackError::NoPreviousImage)?;
        
        // Submit rollback to Axiom
        let proposal = Proposal {
            entry_type: EntryType::ImageRollback,
            payload: ImageRollbackPayload {
                from_image: self.current_image,
                to_image: previous,
                reason: "Boot failure".into(),
            }.into(),
            ..Default::default()
        };
        
        // Rollback is committed even if we can't reach Axiom
        // (will be reconciled on recovery)
        
        self.current_image = previous;
        self.boot_counter = 0;
        
        Ok(())
    }
}
```

---

## Implementation Steps

### Week 1-2: Image Format

1. Define image manifest format
2. Implement image signing
3. Create image verification
4. Build image creation tool

### Week 3-4: Image Manager & Service

1. Implement image staging
2. Add image verification
3. Create upgrade orchestration
4. Implement health checking

### Week 5-6: Bootloader & Testing

1. Implement boot selection
2. Add rollback mechanism
3. Integration testing
4. Failure scenario testing
5. Documentation

---

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| Images verify correctly | Signature test |
| Upgrade is atomic | Crash-during-upgrade test |
| Rollback works | Boot failure test |
| Axiom records upgrade | Audit trail check |

---

*[← Phase 5](05-phase-isolation-tiers.md) | [Phase 7: Bare Metal →](07-phase-bare-metal.md)*
