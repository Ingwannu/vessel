//! Label values shared by the picker and manager; no wire schema is invented here.

use std::sync::Arc;

/// A normalized, nonempty workspace label display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(Arc<str>);

/// A label cannot be created without a visible name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a workspace label name cannot be empty")]
pub struct NameError;

impl Name {
    /// Collapse whitespace and trim as the reference does. No arbitrary cap.
    /// # Errors
    /// Returns `NameError` when only whitespace remains.
    pub fn parse(raw: &str) -> Result<Self, NameError> {
        let text = raw
            .split(reference_whitespace)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            return Err(NameError);
        }
        Ok(Self(text.into()))
    }

    /// Retain host spelling for display and case-only rename.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Workspace identity is case-insensitive, unlike session metadata keys.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        self.key() == other.key()
    }

    pub(super) fn key(&self) -> String {
        self.0.to_lowercase()
    }
}

// JavaScript's \s includes FEFF and excludes 0085; Unicode whitespace differs.
fn reference_whitespace(character: char) -> bool {
    matches!(character, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}'
        | '\u{3000}' | '\u{feff}')
}

/// The reference's finite color choices, independent of theme paint values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Color {
    /// The default create color.
    #[default]
    Violet,
    /// Sky blue.
    Sky,
    /// Emerald green.
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
    /// Palette order is the reference's, not alphabetic order.
    pub const ALL: [Self; 10] = [
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

/// A catalog definition. Assignments carry its name, never a copied color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// Host spelling.
    pub name: Name,
    /// Host-owned color.
    pub color: Color,
}

/// The desired assignment, explicit at every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assignment {
    /// Add to the workspace; may create the catalog definition.
    Assigned,
    /// Remove from the workspace; must not create a definition.
    Unassigned,
}

/// An advisory count from deletion inspection; never a concurrency lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count(pub usize);

/// Correlation identity survives closing/reopening and host changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ticket(u64);

/// A window's ticket source. Exhaustion refuses work rather than reusing identity.
#[derive(Debug, Default)]
pub struct Tickets(u64);

impl Tickets {
    /// Allocate the next operation identity.
    /// # Errors
    /// Refuses identity exhaustion.
    pub fn issue(&mut self) -> Result<Ticket, Failure> {
        self.0 = self.0.checked_add(1).ok_or(Failure::TicketExhausted)?;
        Ok(Ticket(self.0))
    }
}

/// Safe semantic errors; raw RPC diagnostics never become form labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Failure {
    /// The host is not connected.
    #[error("host is offline")]
    Offline,
    /// The node has no assigned contract for this operation.
    #[error("host does not support workspace labels")]
    Unsupported,
    /// Another operation owns the surface or label.
    #[error("a label operation is pending")]
    Pending,
    /// Another definition has that normalized name.
    #[error("a label already uses this name")]
    NameTaken,
    /// The edited definition was removed.
    #[error("label was not found")]
    NotFound,
    /// No valid changed draft exists.
    #[error("there is no valid label change")]
    NoChange,
    /// A validated host response could not be loaded.
    #[error("labels could not be loaded")]
    Load,
    /// The operation failed without a more specific public reason.
    #[error("labels could not be updated")]
    Update,
    /// Every unique identity has been used.
    #[error("label operation identities are exhausted")]
    TicketExhausted,
}
