//! Syscall numbers and error codes for Zero OS

// ============================================================================
// Canonical Syscall Numbers (new ABI)
// ============================================================================

/// Syscall number ranges:
/// - 0x00-0x0F: Misc (debug, info, time)
/// - 0x10-0x1F: Thread (create, exit, yield, sleep)
/// - 0x20-0x2F: Memory (map, unmap, protect)
/// - 0x30-0x3F: Capability (grant, revoke, transfer)
/// - 0x40-0x4F: IPC (send, receive, call, reply)
/// - 0x50-0x5F: IRQ (register, ack, mask)
/// - 0x60-0x6F: I/O (port read/write)
pub mod syscall {
    // === Misc (0x00 - 0x0F) ===
    /// Print debug message
    pub const SYS_DEBUG: u32 = 0x01;
    /// Get current time (arg: 0=low32, 1=high32)
    pub const SYS_GET_TIME: u32 = 0x02;
    /// Get own process ID
    pub const SYS_GET_PID: u32 = 0x03;
    /// List capabilities
    pub const SYS_LIST_CAPS: u32 = 0x04;
    /// List processes
    pub const SYS_LIST_PROCS: u32 = 0x05;
    /// Get wall-clock time in milliseconds since Unix epoch (arg: 0=low32, 1=high32)
    pub const SYS_GET_WALLCLOCK: u32 = 0x06;
    /// Write to console output (for terminal/shell output)
    /// The supervisor receives a notification callback after this syscall completes.
    pub const SYS_CONSOLE_WRITE: u32 = 0x07;

    // === Thread (0x10 - 0x1F) ===
    /// Create a new thread
    pub const SYS_THREAD_CREATE: u32 = 0x10;
    /// Exit current thread/process
    pub const SYS_EXIT: u32 = 0x11;
    /// Yield to scheduler
    pub const SYS_YIELD: u32 = 0x12;
    /// Kill a process (requires Process capability)
    pub const SYS_KILL: u32 = 0x13;
    /// Register a new process (Init-only syscall for spawn protocol)
    pub const SYS_REGISTER_PROCESS: u32 = 0x14;
    /// Create an endpoint for another process (Init-only syscall for spawn protocol)
    pub const SYS_CREATE_ENDPOINT_FOR: u32 = 0x15;
    /// Wait for thread to exit (reusing 0x16 to avoid conflict)
    pub const SYS_THREAD_JOIN: u32 = 0x16;
    /// Sleep for specified nanoseconds (legacy, now 0x15)
    pub const SYS_SLEEP: u32 = 0x15;

    // === Memory (0x20 - 0x2F) ===
    /// Map memory region
    pub const SYS_MMAP: u32 = 0x20;
    /// Unmap memory region
    pub const SYS_MUNMAP: u32 = 0x21;
    /// Change memory protection
    pub const SYS_MPROTECT: u32 = 0x22;

    // === Capability (0x30 - 0x3F) ===
    /// Grant capability to another process
    pub const SYS_CAP_GRANT: u32 = 0x30;
    /// Revoke a capability
    pub const SYS_CAP_REVOKE: u32 = 0x31;
    /// Delete a capability from own CSpace
    pub const SYS_CAP_DELETE: u32 = 0x32;
    /// Inspect a capability
    pub const SYS_CAP_INSPECT: u32 = 0x33;
    /// Derive a capability with reduced permissions
    pub const SYS_CAP_DERIVE: u32 = 0x34;
    /// Create an IPC endpoint
    pub const SYS_EP_CREATE: u32 = 0x35;

    // === IPC (0x40 - 0x4F) ===
    /// Send message to endpoint
    pub const SYS_SEND: u32 = 0x40;
    /// Receive message from endpoint
    pub const SYS_RECEIVE: u32 = 0x41;
    /// Send and wait for reply (RPC)
    pub const SYS_CALL: u32 = 0x42;
    /// Reply to a call
    pub const SYS_REPLY: u32 = 0x43;
    /// Send message with capabilities
    pub const SYS_SEND_CAP: u32 = 0x44;

    // === IRQ (0x50 - 0x5F) ===
    /// Register IRQ handler
    pub const SYS_IRQ_REGISTER: u32 = 0x50;
    /// Acknowledge IRQ
    pub const SYS_IRQ_ACK: u32 = 0x51;
    /// Mask IRQ
    pub const SYS_IRQ_MASK: u32 = 0x52;
    /// Unmask IRQ
    pub const SYS_IRQ_UNMASK: u32 = 0x53;

    // === I/O (0x60 - 0x6F) ===
    /// Read byte from I/O port
    pub const SYS_IO_IN8: u32 = 0x60;
    /// Read word from I/O port
    pub const SYS_IO_IN16: u32 = 0x61;
    /// Read dword from I/O port
    pub const SYS_IO_IN32: u32 = 0x62;
    /// Write byte to I/O port
    pub const SYS_IO_OUT8: u32 = 0x63;
    /// Write word to I/O port
    pub const SYS_IO_OUT16: u32 = 0x64;
    /// Write dword to I/O port
    pub const SYS_IO_OUT32: u32 = 0x65;

    // === Platform Storage (0x70 - 0x7F) ===
    // These are HAL-level key-value storage operations, NOT filesystem operations.
    // VfsService (userspace) uses these syscalls for persistence to IndexedDB/disk.
    // Applications should use zos_vfs::VfsClient for filesystem operations.
    //
    // All storage syscalls are ASYNC and return a request_id immediately.
    // The result is delivered via IPC to the requesting process.

    /// Read blob from platform storage (async - returns request_id)
    /// Args: key_len in data buffer
    /// Returns: request_id (response delivered via IPC with data)
    pub const SYS_STORAGE_READ: u32 = 0x70;

    /// Write blob to platform storage (async - returns request_id)
    /// Args: key_len, value_len in data buffer (key then value)
    /// Returns: request_id (response delivered via IPC with success/error)
    pub const SYS_STORAGE_WRITE: u32 = 0x71;

    /// Delete blob from platform storage (async - returns request_id)
    /// Args: key_len in data buffer
    /// Returns: request_id (response delivered via IPC with success/error)
    pub const SYS_STORAGE_DELETE: u32 = 0x72;

    /// List keys with prefix (async - returns request_id)
    /// Args: prefix_len in data buffer
    /// Returns: request_id (response delivered via IPC with key list)
    pub const SYS_STORAGE_LIST: u32 = 0x73;

    /// Check if key exists (async - returns request_id)
    /// Args: key_len in data buffer
    /// Returns: request_id (response delivered via IPC with bool)
    pub const SYS_STORAGE_EXISTS: u32 = 0x74;

    // === Network (0x90 - 0x9F) ===
    // These are HAL-level HTTP fetch operations for the Network Service.
    // Applications should use the Network Service via IPC (MSG_NET_REQUEST).
    //
    // Network syscalls are ASYNC and return a request_id immediately.
    // The result is delivered via IPC (MSG_NET_RESULT) to the requesting process.

    /// Start async HTTP fetch (returns request_id)
    /// Args: request JSON in data buffer
    /// Returns: request_id (response delivered via IPC with HttpResponse)
    pub const SYS_NETWORK_FETCH: u32 = 0x90;

    // =========================================================================
    // Deprecated VFS syscalls (kept for backward compatibility)
    // These are superseded by the VFS IPC service (zos_vfs::VfsClient)
    // =========================================================================

    /// DEPRECATED: Read file via kernel VFS
    #[deprecated(since = "0.1.0", note = "Use VFS IPC service via zos_vfs::VfsClient")]
    pub const SYS_VFS_READ: u32 = 0x80;

    /// DEPRECATED: Write file via kernel VFS
    #[deprecated(since = "0.1.0", note = "Use VFS IPC service via zos_vfs::VfsClient")]
    pub const SYS_VFS_WRITE: u32 = 0x81;

    /// DEPRECATED: Create directory via kernel VFS
    #[deprecated(since = "0.1.0", note = "Use VFS IPC service via zos_vfs::VfsClient")]
    pub const SYS_VFS_MKDIR: u32 = 0x82;

    /// DEPRECATED: List directory via kernel VFS
    #[deprecated(since = "0.1.0", note = "Use VFS IPC service via zos_vfs::VfsClient")]
    pub const SYS_VFS_LIST: u32 = 0x83;

    /// DEPRECATED: Delete file/directory via kernel VFS
    #[deprecated(since = "0.1.0", note = "Use VFS IPC service via zos_vfs::VfsClient")]
    pub const SYS_VFS_DELETE: u32 = 0x84;

    /// DEPRECATED: Check if path exists via kernel VFS
    #[deprecated(since = "0.1.0", note = "Use VFS IPC service via zos_vfs::VfsClient")]
    pub const SYS_VFS_EXISTS: u32 = 0x85;
}

// ============================================================================
// Error Codes
// ============================================================================

/// Syscall error codes.
pub mod error {
    /// Success
    pub const E_OK: u32 = 0;
    /// Permission denied
    pub const E_PERM: u32 = 1;
    /// Object not found
    pub const E_NOENT: u32 = 2;
    /// Invalid argument
    pub const E_INVAL: u32 = 3;
    /// Syscall not implemented
    pub const E_NOSYS: u32 = 4;
    /// Would block (try again)
    pub const E_AGAIN: u32 = 5;
    /// Out of memory
    pub const E_NOMEM: u32 = 6;
    /// Invalid capability slot
    pub const E_BADF: u32 = 7;
    /// Resource busy
    pub const E_BUSY: u32 = 8;
    /// Already exists
    pub const E_EXIST: u32 = 9;
    /// Buffer overflow
    pub const E_OVERFLOW: u32 = 10;
}
