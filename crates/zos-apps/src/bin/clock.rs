//! Clock Application Binary
//!
//! Entry point for the Clock WASM binary.

#![cfg_attr(target_arch = "wasm32", no_main)]

extern crate alloc;

use zos_apps::app_main;
use zos_apps::apps::ClockApp;

// Entry point
app_main!(ClockApp);

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Clock app is meant to run as WASM in Zero OS");
}
