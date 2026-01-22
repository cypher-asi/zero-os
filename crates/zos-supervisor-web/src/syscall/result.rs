//! Syscall result serialization and delivery
//!
//! Handles sending syscall results back to worker processes.

use zos_kernel::SyscallResult;

use crate::hal::WasmHal;

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Send syscall result back to a Worker via postMessage
///
/// This is used for the legacy postMessage-based syscall path.
pub(crate) fn send_syscall_result(hal: &WasmHal, pid: u64, result: SyscallResult) {
    // Serialize the result
    let result_bytes = serialize_result(result);

    // Post message to Worker
    if let Ok(processes) = hal.processes.lock() {
        if let Some(proc) = processes.get(&pid) {
            let msg = js_sys::Object::new();
            let _ = js_sys::Reflect::set(
                &msg,
                &"type".into(),
                &wasm_bindgen::JsValue::from_str("syscall_result"),
            );
            let _ = js_sys::Reflect::set(&msg, &"result".into(), &wasm_bindgen::JsValue::from(0));
            let array = js_sys::Uint8Array::from(&result_bytes[..]);
            let _ = js_sys::Reflect::set(&msg, &"data".into(), &array);
            let _ = proc.worker.post_message(&msg);
        }
    }
}

/// Serialize a SyscallResult to bytes for transport
fn serialize_result(result: SyscallResult) -> Vec<u8> {
    match result {
        SyscallResult::Ok(value) => {
            let mut bytes = vec![0u8]; // 0 = success
            bytes.extend_from_slice(&value.to_le_bytes());
            bytes
        }
        SyscallResult::Err(e) => {
            let mut bytes = vec![1u8]; // 1 = error
            let err_code = match e {
                zos_kernel::KernelError::ProcessNotFound => 1,
                zos_kernel::KernelError::EndpointNotFound => 2,
                zos_kernel::KernelError::InvalidCapability => 3,
                zos_kernel::KernelError::PermissionDenied => 4,
                zos_kernel::KernelError::WouldBlock => 5,
                zos_kernel::KernelError::Hal(_) => 6,
            };
            bytes.push(err_code);
            bytes
        }
        SyscallResult::WouldBlock => {
            vec![5u8] // 5 = would block
        }
        SyscallResult::Message(msg) => {
            let mut bytes = vec![2u8]; // 2 = message
            bytes.extend_from_slice(&msg.from.0.to_le_bytes());
            bytes.extend_from_slice(&msg.tag.to_le_bytes());
            bytes.extend_from_slice(&(msg.data.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&msg.data);
            bytes
        }
        SyscallResult::CapList(caps) => {
            let mut bytes = vec![3u8]; // 3 = cap list
            bytes.extend_from_slice(&(caps.len() as u32).to_le_bytes());
            for (slot, cap) in caps {
                bytes.extend_from_slice(&slot.to_le_bytes());
                bytes.push(cap.object_type as u8);
                bytes.extend_from_slice(&cap.object_id.to_le_bytes());
            }
            bytes
        }
        SyscallResult::ProcessList(procs) => {
            let mut bytes = vec![4u8]; // 4 = process list
            bytes.extend_from_slice(&(procs.len() as u32).to_le_bytes());
            for (pid, name, _state) in procs {
                bytes.extend_from_slice(&pid.0.to_le_bytes());
                bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
                bytes.extend_from_slice(name.as_bytes());
            }
            bytes
        }
        SyscallResult::MessageWithCaps(msg, slots) => {
            let mut bytes = vec![6u8]; // 6 = message with caps
            bytes.extend_from_slice(&msg.from.0.to_le_bytes());
            bytes.extend_from_slice(&msg.tag.to_le_bytes());
            bytes.extend_from_slice(&(msg.data.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&msg.data);
            // Append capability slots
            bytes.extend_from_slice(&(slots.len() as u32).to_le_bytes());
            for slot in slots {
                bytes.extend_from_slice(&slot.to_le_bytes());
            }
            bytes
        }
        SyscallResult::CapInfo(info) => {
            let mut bytes = vec![7u8]; // 7 = cap info
            bytes.push(info.object_type as u8);
            bytes.push(
                (if info.permissions.read { 1 } else { 0 })
                    | (if info.permissions.write { 2 } else { 0 })
                    | (if info.permissions.grant { 4 } else { 0 }),
            );
            bytes.extend_from_slice(&info.object_id.to_le_bytes());
            bytes.extend_from_slice(&info.id.to_le_bytes());
            bytes.extend_from_slice(&info.generation.to_le_bytes());
            bytes.extend_from_slice(&info.expires_at.to_le_bytes());
            bytes
        }
    }
}
