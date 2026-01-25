//! Shared utilities for the supervisor crate
//!
//! This module provides common utilities used across multiple modules,
//! eliminating code duplication and centralizing maintenance.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Console.log binding for WASM
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

/// Decode a hex string to bytes.
///
/// Returns an error if the hex string has odd length or contains invalid characters.
///
/// # Examples
///
/// ```ignore
/// let bytes = hex_to_bytes("48656c6c6f")?; // "Hello"
/// assert_eq!(bytes, b"Hello");
/// ```
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, &'static str> {
    if !hex.len().is_multiple_of(2) {
        return Err("Invalid hex length");
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| "Invalid hex character"))
        .collect()
}
