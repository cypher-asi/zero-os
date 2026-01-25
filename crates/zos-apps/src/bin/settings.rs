//! Settings Application Binary
//!
//! Entry point for the Settings WASM binary.

#![cfg_attr(target_arch = "wasm32", no_main)]

extern crate alloc;

use zos_apps::app_main;
use zos_apps::apps::SettingsApp;

// Entry point
app_main!(SettingsApp);

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Settings app is meant to run as WASM in Zero OS");
}
