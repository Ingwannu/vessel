// MODIFIED by Vessel (Team-WICKED), 2026-11-15: the environment module
// (spikes/release/patches/README.md, "gpui-pre-windows", change 6); see the
// lines marked "Vessel patch".
#![cfg(target_os = "windows")]

mod clipboard;
mod destination_list;
mod dialog;
mod direct_manipulation;
mod direct_write;
// Vessel patch: telling running programs the environment changed
// (spikes/release/patches/README.md, "gpui-pre-windows", change 6).
mod environment;
mod directx_atlas;
mod directx_devices;
mod directx_renderer;
mod dispatcher;
mod display;
mod events;
mod keyboard;
mod platform;
mod system_notifications;
mod system_settings;
mod util;
mod vsync;
mod window;
mod wrapper;

pub(crate) use clipboard::*;
pub(crate) use destination_list::*;
pub(crate) use direct_write::*;
pub(crate) use directx_atlas::*;
pub(crate) use directx_devices::*;
pub(crate) use directx_renderer::*;
pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub(crate) use events::*;
pub(crate) use keyboard::*;
pub(crate) use platform::*;
pub(crate) use system_notifications::*;
pub(crate) use system_settings::*;
pub(crate) use util::*;
pub(crate) use vsync::*;
pub(crate) use window::*;
pub(crate) use wrapper::*;

pub use platform::WindowsPlatform;
// Vessel patch: see `environment.rs`.
pub use environment::{
    ENVIRONMENT_BROADCAST_TIMEOUT, EnvironmentBroadcastError, broadcast_environment_changed,
};

pub(crate) use windows::Win32::Foundation::HWND;
