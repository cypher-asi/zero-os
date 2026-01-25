//! Terminal Application Binary
//!
//! Entry point for the Terminal WASM binary.

#![cfg_attr(target_arch = "wasm32", no_main)]

extern crate alloc;

use zos_apps::app_main;
use zos_apps::apps::TerminalApp;

// Entry point
app_main!(TerminalApp);

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Terminal app is meant to run as WASM in Zero OS");
}
