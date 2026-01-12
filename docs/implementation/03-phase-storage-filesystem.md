# Phase 3: Storage & Filesystem

**Duration:** 6-8 weeks  
**Status:** Implementation Phase  
**Prerequisites:** Phase 2 (QEMU Kernel)

---

## Objective

Implement user-space block storage and filesystem services with Axiom-backed transactional metadata and content-addressed storage.

---

## Deliverables

### 3.1 Block Driver

| Component | Description | Complexity |
|-----------|-------------|------------|
| virtio-blk driver | QEMU block device | Medium |
| DMA handling | Memory-mapped I/O | Medium |
| Request queue | I/O scheduling | Medium |
| Interrupt handling | Completion notification | Low |

### 3.2 Block Service

| Component | Description | Complexity |
|-----------|-------------|------------|
| Block allocation | Free space management | Medium |
| I/O scheduling | Request ordering | Medium |
| Caching | Block cache | Medium |
| Journal | Write-ahead log | High |

### 3.3 Content Store

| Component | Description | Complexity |
|-----------|-------------|------------|
| Content hashing | BLAKE3 hashing | Low |
| Deduplication | Content-based dedup | Medium |
| Block mapping | Hash → blocks | Medium |
| Garbage collection | Unused block reclaim | High |

### 3.4 Filesystem Service

| Component | Description | Complexity |
|-----------|-------------|------------|
| Namespace manager | Path resolution | Medium |
| Metadata manager | Inode management | High |
| Axiom integration | Transactional metadata | High |
| Directory ops | Create, delete, list | Medium |
| File ops | Read, write, truncate | Medium |

### 3.5 Snapshots

| Component | Description | Complexity |
|-----------|-------------|------------|
| Snapshot creation | Point-in-time copy | Medium |
| Snapshot restoration | Rollback | Medium |
| Snapshot diffing | Compare snapshots | Low |

---

## Technical Approach

### Block Service

```rust
pub struct BlockService {
    /// Block device driver
    driver: BlockDriver,
    
    /// Block allocator
    allocator: BlockAllocator,
    
    /// Block cache
    cache: BlockCache,
    
    /// Write-ahead log
    journal: Journal,
}

impl BlockService {
    pub async fn write(&mut self, offset: u64, data: &[u8]) -> Result<(), BlockError> {
        // Write to journal first
        let journal_entry = self.journal.begin_write(offset, data)?;
        
        // Write to device
        self.driver.write(offset, data).await?;
        
        // Mark journal entry complete
        self.journal.complete(journal_entry)?;
        
        // Update cache
        self.cache.insert(offset, data);
        
        Ok(())
    }
    
    pub async fn read(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, BlockError> {
        // Check cache first
        if let Some(data) = self.cache.get(offset, len) {
            return Ok(data);
        }
        
        // Read from device
        let data = self.driver.read(offset, len).await?;
        
        // Update cache
        self.cache.insert(offset, &data);
        
        Ok(data)
    }
}
```

### Content Store

```rust
pub struct ContentStore {
    /// Block service
    blocks: BlockServiceClient,
    
    /// Content index
    index: ContentIndex,
}

impl ContentStore {
    pub async fn store(&mut self, data: &[u8]) -> Result<Hash, StoreError> {
        let hash = Hash::of(data);
        
        // Check for duplicate
        if self.index.contains(&hash) {
            return Ok(hash);
        }
        
        // Allocate blocks
        let block_count = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let blocks = self.blocks.allocate(block_count as u64).await?;
        
        // Write blocks
        for (i, chunk) in data.chunks(BLOCK_SIZE).enumerate() {
            let offset = blocks.start + i as u64;
            self.blocks.write(offset * BLOCK_SIZE as u64, chunk).await?;
        }
        
        // Update index
        self.index.insert(hash, blocks);
        
        Ok(hash)
    }
    
    pub async fn get(&self, hash: &Hash) -> Result<Vec<u8>, StoreError> {
        let blocks = self.index.get(hash)
            .ok_or(StoreError::NotFound)?;
        
        let mut data = Vec::new();
        for i in 0..blocks.count {
            let offset = (blocks.start + i) * BLOCK_SIZE as u64;
            let block = self.blocks.read(offset, BLOCK_SIZE).await?;
            data.extend_from_slice(&block);
        }
        
        // Verify hash
        if Hash::of(&data) != *hash {
            return Err(StoreError::Corrupted);
        }
        
        Ok(data)
    }
}
```

### Filesystem with Axiom

```rust
pub struct FilesystemService {
    /// Content store
    content: ContentStore,
    
    /// Axiom client
    axiom: AxiomClient,
    
    /// Namespace (in-memory, derived from Axiom)
    namespace: Namespace,
}

impl FilesystemService {
    pub async fn create_file(
        &mut self,
        path: &Path,
        content: &[u8],
    ) -> Result<DirEntry, FsError> {
        // Store content first
        let content_hash = self.content.store(content).await?;
        
        // Prepare Axiom entry
        let proposal = Proposal {
            entry_type: EntryType::FileCreate,
            payload: FileCreatePayload {
                path: path.clone(),
                content_hash,
                size: content.len() as u64,
                permissions: Permissions::default(),
            }.into(),
            ..Default::default()
        };
        
        // Submit to Axiom
        let result = self.axiom.submit(proposal).await?;
        
        // Update namespace
        let entry = DirEntry {
            name: path.file_name().unwrap().to_string(),
            entry_type: EntryType::File,
            content_hash: Some(content_hash),
            metadata: FileMetadata {
                size: content.len() as u64,
                ..Default::default()
            },
            ..Default::default()
        };
        
        self.namespace.insert(path, entry.clone());
        
        Ok(entry)
    }
}
```

---

## Implementation Steps

### Week 1-2: Block Driver & Service

1. Implement virtio-blk driver
2. Add DMA handling
3. Create block service
4. Implement block allocator
5. Add block cache
6. Add journal

### Week 3-4: Content Store

1. Implement content hashing
2. Create content index
3. Add deduplication
4. Implement block mapping
5. Add garbage collection

### Week 5-6: Filesystem Service

1. Implement namespace manager
2. Create metadata structures
3. Add Axiom integration
4. Implement file operations
5. Implement directory operations

### Week 7-8: Snapshots & Testing

1. Implement snapshot creation
2. Add snapshot restoration
3. Integration testing
4. Crash recovery testing
5. Performance optimization

---

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| Block I/O works | Read/write tests |
| Deduplication works | Duplicate content test |
| Metadata is transactional | Crash recovery test |
| Snapshots work | Snapshot/restore test |
| Performance acceptable | Benchmarks |

---

*[← Phase 2](02-phase-qemu-kernel.md) | [Phase 4: Networking →](04-phase-networking.md)*
