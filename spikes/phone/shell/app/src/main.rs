//! The phone app's shell on a desktop: only for looking at it without a
//! phone (`cargo run -p vessel-phone`). Phones enter through `run`.

// No console window on Windows when this is run as an app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    vessel_phone::run();
}
