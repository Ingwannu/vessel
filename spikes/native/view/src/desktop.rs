//! Desktop navigation and window rules; no platform calls or process ownership.

pub mod changelog;
pub mod cli;
mod navigation;
pub mod window;
pub mod zoom;
pub use navigation::{NavigationError, NavigationTarget};
