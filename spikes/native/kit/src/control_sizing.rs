//! Source control-sizing vocabulary, shared by buttons and action triggers.

/// Input layout selected at the app edge, not inferred inside a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlSizing {
    /// Pointer geometry.
    Pointer,
    /// Touch geometry.
    Touch,
}
impl ControlSizing {
    /// Adapt the app's existing compact-layout decision once at the edge.
    #[must_use]
    pub const fn from_compact(narrow: bool) -> Self {
        if narrow { Self::Touch } else { Self::Pointer }
    }
}
/// Paseo's ordinary touch target, kept at one source for all consumers.
pub use crate::breakpoint::TOUCH_TARGET;
