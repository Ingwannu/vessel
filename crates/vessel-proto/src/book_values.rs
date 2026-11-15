//! Shared local client-book appearance values, never RPC or wire fields.
//!
//! [Decision Log]
//! - Purpose: the client file owner and native view need the same validated
//!   Unicode label and closed appearance choices without depending on each
//!   other's I/O or UI layer.
//! - Choice: move the existing pure values here, retaining native old-path
//!   re-exports and tests. No paint tokens, locality inference, persistence,
//!   transport, method, version or TypeScript registration belongs here.
//! - Compatibility: JS trim and the eleven-color/automatic badge catalogue
//!   are unchanged. The file owner enforces its explicit UTF-8 storage cap;
//!   this value never silently truncates a label.

mod label;
mod managed_alias;
pub use label::{EmptyLabel, Label};
pub use managed_alias::ManagedLocalAlias;

/// Paseo's host colors, in `HOST_COLORS` order (none then identity colors).
/// This is a choice, not a paint value; the kit owns its actual colors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Color {
    /// No identity color.
    #[default]
    None,
    /// Violet.
    Violet,
    /// Sky.
    Sky,
    /// Emerald.
    Emerald,
    /// Orange.
    Orange,
    /// Pink.
    Pink,
    /// Indigo.
    Indigo,
    /// Teal.
    Teal,
    /// Red.
    Red,
    /// Amber.
    Amber,
    /// Blue.
    Blue,
}

impl Color {
    /// Choices in the source's stable order.
    pub const ALL: [Self; 11] = [
        Self::None,
        Self::Violet,
        Self::Sky,
        Self::Emerald,
        Self::Orange,
        Self::Pink,
        Self::Indigo,
        Self::Teal,
        Self::Red,
        Self::Amber,
        Self::Blue,
    ];
}

/// The person's choice, kept separately from the automatic default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum BadgePreference {
    /// Local machines hide, remote machines show their name.
    #[default]
    Automatic,
    /// Show the machine's name.
    Name,
    /// Show its icon only.
    Icon,
    /// Hide it regardless of where the machine runs.
    Hidden,
}

#[cfg(test)]
mod tests;
