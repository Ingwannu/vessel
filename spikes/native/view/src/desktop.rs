//! Desktop navigation and window rules; no platform calls or process ownership.

mod navigation;
pub use navigation::{NavigationError, NavigationTarget};
