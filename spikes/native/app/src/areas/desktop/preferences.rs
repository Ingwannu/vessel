//! Browser builds retain their existing in-memory preference path.

#[cfg(not(target_arch="wasm32"))]
pub(crate) use crate::desktop::preferences::{start,submit};

#[cfg(target_arch="wasm32")]
pub(crate) fn start(_shell:&mut crate::shell::Shell,_window:&mut gpui::Window,_cx:&mut gpui::Context<'_,crate::shell::Shell>) {}

#[cfg(target_arch="wasm32")]
pub(crate) fn submit(_shell:&crate::shell::Shell,command:vessel_app_link::Command) -> Option<vessel_app_link::Command> {Some(command)}
