# Storage Service

> Persistent storage access for applications.

## Overview

The Storage Service provides:

1. **Key-Value Storage**: Simple get/set/delete operations
2. **Namespaced Access**: Per-application isolated storage
3. **Quota Management**: Enforce storage limits
4. **Transactions**: Atomic multi-key operations (future)

## WASM Implementation

On WASM, storage is backed by IndexedDB:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Storage Service                               │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    IndexedDB Backend                            ││
│  │                                                                ││
│  │  Database: Zero-storage                                     ││
│  │                                                                ││
│  │  Object Stores:                                                ││
│  │  • system     (system configuration)                           ││
│  │  • app:xyz    (per-app namespace)                              ││
│  │  • user:alice (per-user namespace)                             ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Quota Tracking                               ││
│  │                                                                ││
│  │  Namespace   │ Used    │ Quota   │ % Used                      ││
│  │  ────────────┼─────────┼─────────┼────────                     ││
│  │  system      │ 15 KB   │ 1 MB    │ 1.5%                        ││
│  │  app:xyz     │ 500 KB  │ 10 MB   │ 5%                          ││
│  │  user:alice  │ 2 MB    │ 100 MB  │ 2%                          ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  Message Handlers:                                                   │
│  • STORAGE_GET    → retrieve value                                  │
│  • STORAGE_SET    → store value                                     │
│  • STORAGE_DELETE → remove key                                      │
│  • STORAGE_LIST   → list keys                                       │
│  • STORAGE_QUOTA  → get quota info                                  │
└─────────────────────────────────────────────────────────────────────┘
```

## IPC Protocol

### Get

```rust
/// Storage get request.
pub const MSG_STORAGE_GET: u32 = 0x7000;
/// Storage get response.
pub const MSG_STORAGE_GET_RESPONSE: u32 = 0x7001;

/// Get request.
pub struct StorageGetRequest {
    /// Key to retrieve
    pub key: String,
}

/// Get response.
pub struct StorageGetResponse {
    pub result: Result<Option<Vec<u8>>, StorageError>,
}
```

### Set

```rust
/// Storage set request.
pub const MSG_STORAGE_SET: u32 = 0x7002;
/// Storage set response.
pub const MSG_STORAGE_SET_RESPONSE: u32 = 0x7003;

/// Set request.
pub struct StorageSetRequest {
    /// Key to set
    pub key: String,
    /// Value to store
    pub value: Vec<u8>,
    /// Optional TTL in seconds (0 = no expiry)
    pub ttl_secs: u32,
}

/// Set response.
pub struct StorageSetResponse {
    pub result: Result<(), StorageError>,
}
```

### Delete

```rust
/// Storage delete request.
pub const MSG_STORAGE_DELETE: u32 = 0x7004;
/// Storage delete response.
pub const MSG_STORAGE_DELETE_RESPONSE: u32 = 0x7005;

/// Delete request.
pub struct StorageDeleteRequest {
    pub key: String,
}

/// Delete response.
pub struct StorageDeleteResponse {
    pub result: Result<bool, StorageError>,  // true if key existed
}
```

### List Keys

```rust
/// Storage list request.
pub const MSG_STORAGE_LIST: u32 = 0x7006;
/// Storage list response.
pub const MSG_STORAGE_LIST_RESPONSE: u32 = 0x7007;

/// List request.
pub struct StorageListRequest {
    /// Key prefix to match
    pub prefix: String,
    /// Maximum keys to return
    pub limit: usize,
    /// Cursor for pagination
    pub cursor: Option<String>,
}

/// List response.
pub struct StorageListResponse {
    pub keys: Vec<String>,
    pub next_cursor: Option<String>,
}
```

### Errors

```rust
#[derive(Clone, Debug)]
pub enum StorageError {
    /// Key not found
    NotFound,
    /// Quota exceeded
    QuotaExceeded,
    /// Key too long
    KeyTooLong,
    /// Value too large
    ValueTooLarge,
    /// Permission denied
    PermissionDenied,
    /// Backend error
    BackendError(String),
}
```

## Namespace Isolation

Each process accesses a namespaced portion of storage:

```rust
impl StorageService {
    fn resolve_key(&self, caller: ProcessId, key: &str) -> String {
        let namespace = self.get_namespace(caller);
        format!("{}:{}", namespace, key)
    }
    
    fn get_namespace(&self, pid: ProcessId) -> String {
        // Look up process info to determine namespace
        let info = self.get_process_info(pid);
        
        // System services get "system" namespace
        if info.roles.contains(&"system".to_string()) {
            return "system".to_string();
        }
        
        // Apps get per-app namespace
        format!("app:{}", info.name)
    }
}
```

## Quota Management

```rust
/// Per-namespace quota tracking.
struct QuotaTracker {
    /// Namespace -> (used bytes, max bytes)
    quotas: BTreeMap<String, (usize, usize)>,
}

impl QuotaTracker {
    fn check_quota(&self, namespace: &str, additional: usize) -> Result<(), StorageError> {
        let (used, max) = self.quotas.get(namespace)
            .unwrap_or(&(0, DEFAULT_QUOTA));
        
        if used + additional > *max {
            return Err(StorageError::QuotaExceeded);
        }
        
        Ok(())
    }
    
    fn update_usage(&mut self, namespace: &str, delta: isize) {
        let entry = self.quotas.entry(namespace.to_string())
            .or_insert((0, DEFAULT_QUOTA));
        
        entry.0 = (entry.0 as isize + delta).max(0) as usize;
    }
}

/// Default quota per namespace (10 MB).
const DEFAULT_QUOTA: usize = 10 * 1024 * 1024;

/// System namespace quota (1 MB).
const SYSTEM_QUOTA: usize = 1024 * 1024;

/// Maximum key length.
const MAX_KEY_LENGTH: usize = 256;

/// Maximum value size.
const MAX_VALUE_SIZE: usize = 1024 * 1024;  // 1 MB
```

## IndexedDB Backend (WASM)

```javascript
// storage_backend.js

class StorageBackend {
    constructor() {
        this.db = null;
    }
    
    async init() {
        const request = indexedDB.open('Zero-storage', 1);
        
        request.onupgradeneeded = (event) => {
            const db = event.target.result;
            
            // Create object stores for each namespace type
            db.createObjectStore('system', { keyPath: 'key' });
            db.createObjectStore('apps', { keyPath: 'key' });
            db.createObjectStore('users', { keyPath: 'key' });
        };
        
        this.db = await new Promise((resolve, reject) => {
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    }
    
    async get(namespace, key) {
        const store = this.getStore(namespace);
        const tx = this.db.transaction(store, 'readonly');
        const objectStore = tx.objectStore(store);
        
        return new Promise((resolve, reject) => {
            const request = objectStore.get(key);
            request.onsuccess = () => resolve(request.result?.value);
            request.onerror = () => reject(request.error);
        });
    }
    
    async set(namespace, key, value, ttl) {
        const store = this.getStore(namespace);
        const tx = this.db.transaction(store, 'readwrite');
        const objectStore = tx.objectStore(store);
        
        const entry = {
            key,
            value,
            expires: ttl ? Date.now() + ttl * 1000 : null,
            created: Date.now(),
        };
        
        return new Promise((resolve, reject) => {
            const request = objectStore.put(entry);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }
    
    getStore(namespace) {
        if (namespace === 'system') return 'system';
        if (namespace.startsWith('app:')) return 'apps';
        if (namespace.startsWith('user:')) return 'users';
        throw new Error(`Unknown namespace: ${namespace}`);
    }
}
```

## Native Backend (Future)

On native targets, storage uses the filesystem:

```rust
// native_storage.rs (future)

struct NativeStorage {
    base_path: PathBuf,
}

impl NativeStorage {
    fn key_to_path(&self, namespace: &str, key: &str) -> PathBuf {
        // Sanitize key to prevent path traversal
        let safe_key = sanitize_filename(key);
        self.base_path.join(namespace).join(safe_key)
    }
    
    fn get(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let path = self.key_to_path(namespace, key);
        
        match fs::read(&path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(StorageError::BackendError(e.to_string())),
        }
    }
    
    fn set(&self, namespace: &str, key: &str, value: &[u8]) -> Result<(), StorageError> {
        let path = self.key_to_path(namespace, key);
        
        // Create namespace directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write atomically (write to temp, then rename)
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, value)?;
        fs::rename(&temp_path, &path)?;
        
        Ok(())
    }
}
```

## WASM Implementation

```rust
// storage_service.rs

#![no_std]
extern crate alloc;
extern crate Zero_process;

use Zero_process::*;

#[no_mangle]
pub extern "C" fn _start() {
    debug("storage: starting");
    
    // Initialize IndexedDB backend (via JS)
    init_backend();
    
    let service_ep = create_endpoint();
    register_service("storage", service_ep);
    send_ready();
    
    loop {
        let msg = receive_blocking(service_ep);
        match msg.tag {
            MSG_STORAGE_GET => handle_get(msg),
            MSG_STORAGE_SET => handle_set(msg),
            MSG_STORAGE_DELETE => handle_delete(msg),
            MSG_STORAGE_LIST => handle_list(msg),
            MSG_STORAGE_QUOTA => handle_quota(msg),
            _ => debug("storage: unknown message"),
        }
    }
}

fn handle_get(msg: ReceivedMessage) {
    let request: StorageGetRequest = decode(&msg.data);
    let namespace = get_namespace(msg.from);
    
    // Check permission
    if !check_read_permission(msg.from, &namespace) {
        send_error(msg, StorageError::PermissionDenied);
        return;
    }
    
    // Resolve and get
    let full_key = format!("{}:{}", namespace, request.key);
    let result = backend_get(&full_key);
    
    send_response(msg, StorageGetResponse { result });
}
```
