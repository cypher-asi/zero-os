//! Axiom IndexedDB persistence
//!
//! Handles syncing the kernel's CommitLog and SysLog to browser IndexedDB.

use wasm_bindgen::prelude::*;

use super::{log, Supervisor};
use crate::bindings::axiom_storage;

/// Internal async helper for axiom initialization.
/// This is a standalone async function that doesn't hold any borrows,
/// avoiding wasm-bindgen closure issues with &mut self across await points.
async fn do_axiom_init() -> Option<(u64, u64)> {
    let result = axiom_storage::init().await;
    if !result.is_truthy() {
        log("[axiom] Failed to initialize IndexedDB");
        return None;
    }

    let last_seq = axiom_storage::getLastSeq().await;
    let count = axiom_storage::getCount().await;

    let seq_num = last_seq.as_f64().map(|s| if s < 0.0 { 0 } else { s as u64 + 1 });
    let count_num = count.as_f64().map(|n| n as u64);

    Some((seq_num.unwrap_or(0), count_num.unwrap_or(0)))
}

#[wasm_bindgen]
impl Supervisor {
    /// Initialize Axiom storage (IndexedDB) - call this before boot()
    /// Returns a Promise that resolves when storage is ready
    ///
    /// NOTE: This method avoids the wasm-bindgen "closure invoked recursively or
    /// after being dropped" error by:
    /// 1. Checking for double initialization upfront (synchronous guard)
    /// 2. Performing all async IndexedDB operations in a separate async function
    ///    that doesn't hold &mut self across await points
    /// 3. Updating state synchronously after the async work completes
    #[wasm_bindgen]
    pub async fn init_axiom_storage(&mut self) -> Result<JsValue, JsValue> {
        // Guard against double initialization (synchronous check)
        if self.axiom_storage_ready {
            log("[axiom] Already initialized");
            return Ok(JsValue::from_bool(true));
        }

        log("[axiom] Initializing IndexedDB storage...");

        // Perform all async work in a standalone function that doesn't borrow self.
        // This avoids wasm-bindgen issues with holding &mut self across await points.
        let result = do_axiom_init().await;

        // Update state synchronously based on result
        match result {
            Some((last_seq, count)) => {
                self.axiom_storage_ready = true;
                self.last_persisted_axiom_seq = last_seq;
                log(&format!("[axiom] Storage ready, last_seq={}", last_seq));
                log(&format!("[axiom] {} entries in IndexedDB", count));
                Ok(JsValue::from_bool(true))
            }
            None => Ok(JsValue::from_bool(false)),
        }
    }

    /// Sync new CommitLog entries to IndexedDB
    /// Call this periodically (e.g., after each command or in main loop)
    /// Returns the number of entries synced
    #[wasm_bindgen]
    pub async fn sync_axiom_log(&mut self) -> u32 {
        if !self.axiom_storage_ready {
            return 0;
        }

        let commitlog = self.system.commitlog();
        let current_seq = commitlog.current_seq() + 1;

        // Nothing new to persist
        if current_seq <= self.last_persisted_axiom_seq {
            return 0;
        }

        // Get commits to persist
        let commits_to_persist: Vec<_> = commitlog
            .commits()
            .iter()
            .filter(|c| c.seq >= self.last_persisted_axiom_seq)
            .collect();

        if commits_to_persist.is_empty() {
            return 0;
        }

        // Convert to JS array (synchronous operation)
        let js_entries = js_sys::Array::new();
        for commit in &commits_to_persist {
            js_entries.push(&axiom_storage::commit_to_js(commit));
        }

        // Persist to IndexedDB - the only await point
        // Note: We prepare all data before awaiting to minimize borrow duration issues
        let result = axiom_storage::persistEntries(js_entries.into()).await;
        
        // Update state synchronously after await
        if let Some(count) = result.as_f64() {
            let persisted = count as u32;
            self.last_persisted_axiom_seq = current_seq;
            if persisted > 0 {
                log(&format!(
                    "[axiom] Persisted {} commits to IndexedDB (seq now {})",
                    persisted, current_seq
                ));
            }
            persisted
        } else {
            log("[axiom] Failed to persist commits");
            0
        }
    }

    /// Clear all Axiom log entries from IndexedDB (for testing/reset)
    #[wasm_bindgen]
    pub async fn clear_axiom_storage(&mut self) -> bool {
        if !self.axiom_storage_ready {
            return false;
        }

        let result = axiom_storage::clear().await;
        if result.is_undefined() || result.is_null() {
            self.last_persisted_axiom_seq = 0;
            log("[axiom] Cleared IndexedDB storage");
            true
        } else {
            false
        }
    }
}
