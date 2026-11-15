//! Desktop navigation and window rules; no platform calls or process ownership.

pub mod changelog;
pub mod cli;
mod navigation;
pub mod window;
pub mod zoom;
pub use navigation::{NavigationError, NavigationTarget};

/// The selection owner decides whether a context menu may copy anything.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CopyState {
    /// A field or terminal has a copyable selection.
    Available,
    /// No selection exists; the menu keeps Copy visible but disabled.
    #[default]
    Unavailable,
}
