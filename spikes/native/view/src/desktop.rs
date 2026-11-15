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

/// Read-only terminals keep selection actions while refusing input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PasteState {
    /// The focused owner accepts pasted input.
    Available,
    /// Copy and selection remain usable without an input action.
    #[default]
    Unavailable,
}

/// Selection and input authority come from the field or terminal owner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditContext {
    /// Whether a copyable selection exists.
    pub copy: CopyState,
    /// Whether the owner accepts input (scripts/services may not).
    pub paste: PasteState,
}

/// Shared Help/sidebar destinations, resolved once by the desktop screen owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpAction {
    /// The shared release history sheet.
    Changelog,
    /// About shows the running app version and update controls.
    About,
    /// Open the project's issue form through the toolkit's external opener.
    ReportIssue,
}
