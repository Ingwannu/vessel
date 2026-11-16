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

// Source fieldControlMd: (height - line20 - two borders) / two sides.
// Shared by select and multiline field chrome; compact GOAT insets differ.
const MD_FIELD_POINTER_VERTICAL: f32 = 5.0;
const MD_FIELD_TOUCH_VERTICAL: f32 = 11.0;
pub(crate) const fn md_field_vertical(sizing: ControlSizing) -> f32 {
    match sizing {
        ControlSizing::Pointer => MD_FIELD_POINTER_VERTICAL,
        ControlSizing::Touch => MD_FIELD_TOUCH_VERTICAL,
    }
}
