//! Desktop navigation and window rules; no platform calls or process ownership.

pub mod changelog;
mod navigation;
pub mod window;
pub use navigation::{NavigationError, NavigationTarget};
