//! Host functions for WASM processes
//!
//! These functions are imported by WASM processes to communicate with the kernel.
//! They match the interface defined in `zos-process/src/syscalls/mod.rs`.

use alloc::vec::Vec;
use wasmi::{Caller, Linker};

use super::serial;

/// Host state for a WASM process
///
/// Contains the process's syscall buffers and state.
pub struct HostState {
    /// Process ID
    pub pid: u64,
    /// Syscall input buffer (data sent by process)
    pub syscall_in_buffer: Vec<u8>,
    /// Syscall output buffer (result data for process)
    pub syscall_out_buffer: Vec<u8>,
    /// Last syscall result code
    pub syscall_result: u32,
    /// Whether a result is pending from kernel (syscall was processed)
    pub has_pending_result: bool,
    /// Process is waiting for a syscall result
    pub waiting_for_syscall: bool,
    /// Process has yielded
    pub yielded: bool,
    /// Pending syscall to dispatch
    pub pending_syscall: Option<PendingSyscallInfo>,
}

/// Information about a pending syscall
#[derive(Clone, Debug)]
pub struct PendingSyscallInfo {
    pub syscall_num: u32,
    pub args: [u32; 3],
    pub data: Vec<u8>,
}

impl HostState {
    /// Create new host state for a process
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            syscall_in_buffer: Vec::with_capacity(super::MAX_SYSCALL_BUFFER),
            syscall_out_buffer: Vec::new(),
            syscall_result: 0,
            has_pending_result: false,
            waiting_for_syscall: false,
            yielded: false,
            pending_syscall: None,
        }
    }
    
    /// Set syscall result (called by kernel after processing syscall)
    pub fn set_syscall_result(&mut self, result: u32, data: &[u8]) {
        self.syscall_result = result;
        self.syscall_out_buffer.clear();
        self.syscall_out_buffer.extend_from_slice(data);
        self.has_pending_result = true;
        self.waiting_for_syscall = false;
    }
    
    /// Clear syscall buffers
    pub fn clear_syscall_buffers(&mut self) {
        self.syscall_in_buffer.clear();
        self.syscall_out_buffer.clear();
        self.pending_syscall = None;
        self.has_pending_result = false;
        self.waiting_for_syscall = false;
    }
}

// Syscall numbers (from zos-ipc)
const SYS_NOP: u32 = 0x00;
const SYS_DEBUG: u32 = 0x01;
const SYS_TIME: u32 = 0x02;
const SYS_GETPID: u32 = 0x03;
const SYS_YIELD: u32 = 0x04;
const SYS_RANDOM: u32 = 0x05;
const SYS_CONSOLE_WRITE: u32 = 0x07;

/// Register host functions with the linker
pub fn register_host_functions(linker: &mut Linker<HostState>) {
    // Register wasm-bindgen shims first (these are no-ops but required for linking)
    register_wasm_bindgen_shims(linker);
    // zos_syscall(syscall_num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32
    // Returns Result to allow triggering resumable pauses for syscalls that need kernel processing
    linker.func_wrap("env", "zos_syscall", |mut caller: Caller<'_, HostState>, syscall_num: u32, arg1: u32, arg2: u32, arg3: u32| -> Result<u32, wasmi::Error> {
        let host = caller.data_mut();
        let pid = host.pid;
        
        // Handle simple syscalls directly without going through kernel
        match syscall_num {
            SYS_NOP => return Ok(0),
            
            SYS_DEBUG | SYS_CONSOLE_WRITE => {
                // Print debug/console output directly to serial
                let data = core::mem::take(&mut host.syscall_in_buffer);
                if let Ok(text) = core::str::from_utf8(&data) {
                    serial::write_str(text);
                }
                return Ok(0);
            }
            
            SYS_GETPID => return Ok(pid as u32),
            
            SYS_RANDOM => {
                // Generate random bytes using RDRAND
                let requested = core::cmp::min(arg1 as usize, 256);
                let mut random_bytes = alloc::vec![0u8; requested];
                
                // Use the HAL's random_bytes function via RDRAND
                use crate::x86_64::random::fill_random_bytes;
                if fill_random_bytes(&mut random_bytes) {
                    // Store in output buffer for process to retrieve
                    host.syscall_out_buffer.clear();
                    host.syscall_out_buffer.extend_from_slice(&random_bytes);
                    return Ok(requested as u32);
                } else {
                    return Ok(0xFFFFFFFF); // Error: RDRAND not available
                }
            }
            
            SYS_YIELD => {
                // Mark as yielded and trigger a resumable pause
                host.yielded = true;
                // Return an error to trigger a resumable pause from the host
                // This allows wasmi to return Resumable instead of just continuing execution
                return Err(wasmi::Error::from(wasmi::core::TrapCode::OutOfFuel));
            }
            
            _ => {
                // Other syscalls need kernel processing
                // Don't log SYS_RECV (0x41) to avoid spamming console during idle loop
                if syscall_num != 0x41 {
                    serial::write_str(&alloc::format!(
                        "[wasm-rt] PID {} syscall: num=0x{:x}, args=[{}, {}, {}]\n",
                        pid, syscall_num, arg1, arg2, arg3
                    ));
                }
            }
        }
        
        // Check if we already have a result waiting (syscall was processed, we're resuming)
        if host.has_pending_result {
            // We have a result from kernel - return it and clear for next syscall
            let result = host.syscall_result;
            host.syscall_result = 0;
            host.has_pending_result = false;
            // Don't clear syscall_out_buffer - process may call zos_recv_bytes to get it
            return Ok(result);
        }
        
        // Store pending syscall for the kernel to process
        host.pending_syscall = Some(PendingSyscallInfo {
            syscall_num,
            args: [arg1, arg2, arg3],
            data: core::mem::take(&mut host.syscall_in_buffer),
        });
        
        // Mark that we need to wait for a syscall result
        host.waiting_for_syscall = true;
        
        // Trigger a resumable pause by returning an error from the host function
        // This allows wasmi to return Resumable (host trap) instead of Err (wasm trap)
        Err(wasmi::Error::from(wasmi::core::TrapCode::OutOfFuel))
    }).expect("Failed to register zos_syscall");
    
    // zos_send_bytes(ptr: u32, len: u32)
    linker.func_wrap("env", "zos_send_bytes", |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| {
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => {
                serial::write_str("[wasm-rt] ERROR: No memory export\n");
                return;
            }
        };
        
        let start = ptr as usize;
        let end = start + len as usize;
        
        // Read bytes from WASM memory
        let bytes: Vec<u8> = {
            let data = memory.data(&caller);
            if end > data.len() {
                serial::write_str(&alloc::format!(
                    "[wasm-rt] ERROR: zos_send_bytes out of bounds: {}..{} > {}\n",
                    start, end, data.len()
                ));
                return;
            }
            data[start..end].to_vec()
        };
        
        // Now we can safely borrow host mutably
        let host = caller.data_mut();
        host.syscall_in_buffer.clear();
        host.syscall_in_buffer.extend_from_slice(&bytes);
    }).expect("Failed to register zos_send_bytes");
    
    // zos_recv_bytes(ptr: u32, max_len: u32) -> u32
    linker.func_wrap("env", "zos_recv_bytes", |mut caller: Caller<'_, HostState>, ptr: u32, max_len: u32| -> u32 {
        let out_data: Vec<u8>;
        {
            let host = caller.data();
            out_data = host.syscall_out_buffer.clone();
        }
        
        let copy_len = core::cmp::min(out_data.len(), max_len as usize);
        
        if copy_len == 0 {
            return 0;
        }
        
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => {
                serial::write_str("[wasm-rt] ERROR: No memory export\n");
                return 0;
            }
        };
        
        let data = memory.data_mut(&mut caller);
        let start = ptr as usize;
        let end = start + copy_len;
        
        if end > data.len() {
            serial::write_str(&alloc::format!(
                "[wasm-rt] ERROR: zos_recv_bytes out of bounds: {}..{} > {}\n",
                start, end, data.len()
            ));
            return 0;
        }
        
        data[start..end].copy_from_slice(&out_data[..copy_len]);
        copy_len as u32
    }).expect("Failed to register zos_recv_bytes");
    
    // zos_yield()
    linker.func_wrap("env", "zos_yield", |mut caller: Caller<'_, HostState>| {
        let host = caller.data_mut();
        host.yielded = true;
        // In a full implementation, this would trap to return control to scheduler
    }).expect("Failed to register zos_yield");
    
    // zos_get_pid() -> u32
    linker.func_wrap("env", "zos_get_pid", |caller: Caller<'_, HostState>| -> u32 {
        caller.data().pid as u32
    }).expect("Failed to register zos_get_pid");
}

/// Register wasm-bindgen stub functions
///
/// These functions are required when the WASM module was compiled with wasm-bindgen
/// (e.g., for getrandom's "js" feature). In QEMU mode, we provide no-op stubs since
/// the actual random generation uses the SYS_RANDOM syscall via RDRAND.
fn register_wasm_bindgen_shims(linker: &mut Linker<HostState>) {
    // __wbindgen_placeholder__::__wbindgen_describe is used for type introspection
    // at link time. We provide a no-op since we don't need JS type info.
    linker.func_wrap("__wbindgen_placeholder__", "__wbindgen_describe", |_: i32| {
        // No-op: type description is only used by JS glue
    }).expect("Failed to register __wbindgen_describe");
    
    // __wbindgen_throw - register in __wbindgen_placeholder__ module
    // Newer wasm-bindgen versions look for this here instead of in wbg module
    linker.func_wrap("__wbindgen_placeholder__", "__wbindgen_throw", |_caller: Caller<'_, HostState>, _ptr: i32, _len: i32| {
        serial::write_str("[wasm-rt] __wbindgen_throw called (placeholder module)\n");
    }).ok();
    
    // Mangled variant of __wbindgen_throw used by some wasm-bindgen generated code
    linker.func_wrap("__wbindgen_placeholder__", "__wbg___wbindgen_throw_be289d5034ed271b", |_caller: Caller<'_, HostState>, _ptr: i32, _len: i32| {
        serial::write_str("[wasm-rt] __wbindgen_throw called (mangled variant)\n");
    }).ok();
    
    // __wbindgen_externref_xform__::__wbindgen_externref_table_grow is for externref tables
    linker.func_wrap("__wbindgen_externref_xform__", "__wbindgen_externref_table_grow", |_: i32| -> i32 {
        0 // Return 0 (no growth needed)
    }).ok(); // Optional - may not be present in all modules
    
    // __wbindgen_externref_xform__::__wbindgen_externref_table_set_null
    linker.func_wrap("__wbindgen_externref_xform__", "__wbindgen_externref_table_set_null", |_: i32| {
        // No-op
    }).ok();
    
    // wbg namespace functions for crypto.getRandomValues
    // These are called by getrandom with the "js" feature
    linker.func_wrap("wbg", "__wbg_crypto_1d1f22824a6a080c", |_: i32| -> i32 {
        // Return a "handle" to crypto object - we'll use 1 as a sentinel
        1
    }).ok();
    
    linker.func_wrap("wbg", "__wbg_getRandomValues_37fa2ca9e4e07fab", |mut caller: Caller<'_, HostState>, _obj: i32, ptr: i32, len: i32| {
        // Fill the buffer with random values using RDRAND
        if len <= 0 {
            return;
        }
        
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => {
                serial::write_str("[wasm-rt] ERROR: __wbg_getRandomValues - no memory export\n");
                return;
            }
        };
        
        // Generate random bytes
        let len = len as usize;
        let mut random_bytes = alloc::vec![0u8; len];
        
        use crate::x86_64::random::fill_random_bytes;
        if !fill_random_bytes(&mut random_bytes) {
            serial::write_str("[wasm-rt] ERROR: RDRAND failed in __wbg_getRandomValues\n");
            return;
        }
        
        // Write to WASM memory
        let data = memory.data_mut(&mut caller);
        let start = ptr as usize;
        let end = start + len;
        
        if end > data.len() {
            serial::write_str("[wasm-rt] ERROR: __wbg_getRandomValues out of bounds\n");
            return;
        }
        
        data[start..end].copy_from_slice(&random_bytes);
    }).ok();
    
    // Common wasm-bindgen exports that might be imported
    linker.func_wrap("wbg", "__wbindgen_object_drop_ref", |_: i32| {
        // No-op: we don't manage JS object references
    }).ok();
    
    linker.func_wrap("wbg", "__wbindgen_throw", |_caller: Caller<'_, HostState>, _ptr: i32, _len: i32| {
        serial::write_str("[wasm-rt] __wbindgen_throw called - JS exception\n");
    }).ok();
    
    linker.func_wrap("wbg", "__wbindgen_is_undefined", |_: i32| -> i32 {
        1 // Everything is "undefined" in QEMU mode
    }).ok();
    
    linker.func_wrap("wbg", "__wbindgen_is_null", |_: i32| -> i32 {
        0
    }).ok();
    
    linker.func_wrap("wbg", "__wbindgen_is_object", |_: i32| -> i32 {
        0
    }).ok();
    
    linker.func_wrap("wbg", "__wbindgen_is_function", |_: i32| -> i32 {
        0
    }).ok();
}
