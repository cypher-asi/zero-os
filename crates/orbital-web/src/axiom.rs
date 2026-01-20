//! Axiom Storage - IndexedDB persistence for WASM targets

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// AxiomStorage JavaScript object for IndexedDB persistence
    #[wasm_bindgen(js_namespace = AxiomStorage)]
    pub async fn init() -> JsValue;

    #[wasm_bindgen(js_namespace = AxiomStorage)]
    pub async fn persistEntry(entry: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = AxiomStorage)]
    pub async fn persistEntries(entries: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = AxiomStorage)]
    pub async fn loadAll() -> JsValue;

    #[wasm_bindgen(js_namespace = AxiomStorage)]
    pub async fn getCount() -> JsValue;

    #[wasm_bindgen(js_namespace = AxiomStorage)]
    pub async fn getLastSeq() -> JsValue;

    #[wasm_bindgen(js_namespace = AxiomStorage)]
    pub async fn clear() -> JsValue;
}

/// Serialize a Commit entry to a JavaScript object for IndexedDB storage
pub(crate) fn commit_to_js(commit: &orbital_kernel::Commit) -> JsValue {
    let obj = js_sys::Object::new();

    // seq: u64 (as number - safe up to 2^53)
    let _ = js_sys::Reflect::set(&obj, &"seq".into(), &JsValue::from_f64(commit.seq as f64));

    // timestamp: u64 (as number)
    let _ = js_sys::Reflect::set(
        &obj,
        &"timestamp".into(),
        &JsValue::from_f64(commit.timestamp as f64),
    );

    // id: [u8; 32] (as hex string)
    let id_hex: String = commit.id.iter().map(|b| format!("{:02x}", b)).collect();
    let _ = js_sys::Reflect::set(&obj, &"id".into(), &JsValue::from_str(&id_hex));

    // prev_commit: [u8; 32] (as hex string)
    let prev_hex: String = commit
        .prev_commit
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    let _ = js_sys::Reflect::set(&obj, &"prev_commit".into(), &JsValue::from_str(&prev_hex));

    // commit_type: CommitType (as type string)
    let commit_type = commit_type_to_string(&commit.commit_type);
    let _ = js_sys::Reflect::set(
        &obj,
        &"commit_type".into(),
        &JsValue::from_str(&commit_type),
    );

    obj.into()
}

/// Convert CommitType to a human-readable string
pub(crate) fn commit_type_to_string(ct: &orbital_kernel::CommitType) -> String {
    match ct {
        orbital_kernel::CommitType::Genesis => "Genesis".to_string(),
        orbital_kernel::CommitType::ProcessCreated { pid, parent, name } => format!(
            "ProcessCreated(pid={}, parent={}, name={})",
            pid, parent, name
        ),
        orbital_kernel::CommitType::ProcessExited { pid, code } => {
            format!("ProcessExited(pid={}, code={})", pid, code)
        }
        orbital_kernel::CommitType::CapInserted {
            pid, slot, cap_id, ..
        } => format!("CapInserted(pid={}, slot={}, cap={})", pid, slot, cap_id),
        orbital_kernel::CommitType::CapRemoved { pid, slot } => {
            format!("CapRemoved(pid={}, slot={})", pid, slot)
        }
        orbital_kernel::CommitType::CapGranted {
            from_pid,
            to_pid,
            from_slot,
            to_slot,
            ..
        } => format!(
            "CapGranted(from={}.{} to={}.{})",
            from_pid, from_slot, to_pid, to_slot
        ),
        orbital_kernel::CommitType::EndpointCreated { id, owner } => {
            format!("EndpointCreated(id={}, owner={})", id, owner)
        }
        orbital_kernel::CommitType::EndpointDestroyed { id } => {
            format!("EndpointDestroyed(id={})", id)
        }
    }
}

/// Get short commit type name for dashboard display
pub(crate) fn commit_type_short(ct: &orbital_kernel::CommitType) -> &'static str {
    match ct {
        orbital_kernel::CommitType::Genesis => "Genesis",
        orbital_kernel::CommitType::ProcessCreated { .. } => "ProcCreate",
        orbital_kernel::CommitType::ProcessExited { .. } => "ProcExit",
        orbital_kernel::CommitType::CapInserted { .. } => "CapInsert",
        orbital_kernel::CommitType::CapRemoved { .. } => "CapRemove",
        orbital_kernel::CommitType::CapGranted { .. } => "CapGrant",
        orbital_kernel::CommitType::EndpointCreated { .. } => "EpCreate",
        orbital_kernel::CommitType::EndpointDestroyed { .. } => "EpDestroy",
    }
}
