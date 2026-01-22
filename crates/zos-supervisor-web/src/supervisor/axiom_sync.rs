//! Axiom IndexedDB persistence
//!
//! Handles syncing the kernel's CommitLog and SysLog to browser IndexedDB.

use wasm_bindgen::prelude::*;

use super::{log, Supervisor};
use crate::axiom;

#[wasm_bindgen]
impl Supervisor {
    /// Initialize Axiom storage (IndexedDB) - call this before boot()
    /// Returns a Promise that resolves when storage is ready
    #[wasm_bindgen]
    pub async fn init_axiom_storage(&mut self) -> Result<JsValue, JsValue> {
        log("[axiom] Initializing IndexedDB storage...");

        let result = axiom::init().await;

        if result.is_truthy() {
            self.axiom_storage_ready = true;

            // Get the last persisted sequence number
            let last_seq = axiom::getLastSeq().await;
            if let Some(seq) = last_seq.as_f64() {
                self.last_persisted_axiom_seq = if seq < 0.0 { 0 } else { seq as u64 + 1 };
                log(&format!(
                    "[axiom] Storage ready, last_seq={}",
                    self.last_persisted_axiom_seq
                ));
            }

            // Get count of stored entries
            let count = axiom::getCount().await;
            if let Some(n) = count.as_f64() {
                log(&format!("[axiom] {} entries in IndexedDB", n as u64));
            }

            Ok(JsValue::from_bool(true))
        } else {
            log("[axiom] Failed to initialize IndexedDB");
            Ok(JsValue::from_bool(false))
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

        let commitlog = self.kernel.commitlog();
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

        // Convert to JS array
        let js_entries = js_sys::Array::new();
        for commit in &commits_to_persist {
            js_entries.push(&axiom::commit_to_js(commit));
        }

        // Persist to IndexedDB
        let result = axiom::persistEntries(js_entries.into()).await;
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

        let result = axiom::clear().await;
        if result.is_undefined() || result.is_null() {
            self.last_persisted_axiom_seq = 0;
            log("[axiom] Cleared IndexedDB storage");
            true
        } else {
            false
        }
    }
}
