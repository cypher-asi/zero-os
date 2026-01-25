//! Calculator Application Binary
//!
//! Entry point for the Calculator WASM binary.

#![cfg_attr(target_arch = "wasm32", no_main)]

extern crate alloc;

use zos_apps::app_main;
use zos_apps::apps::CalculatorApp;

// Entry point
app_main!(CalculatorApp);

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Calculator app is meant to run as WASM in Zero OS");
}
