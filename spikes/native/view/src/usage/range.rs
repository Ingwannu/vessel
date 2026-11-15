//! Settings supports rolling machine periods; session lifetime is a different scope.
use crate::{connection::ConnectionContext, localization::Translator};

/// Machine totals may never accidentally request a session-only interval.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Range {
    /// The node's rolling day, not the viewer's calendar day.
    #[default]
    Day,
    /// The node's rolling week.
    Week,
}
impl Range {
    /// Stable order of the shared segmented control.
    pub const ALL: [Self; 2] = [Self::Day, Self::Week];
    /// Labels explicitly name the rolling interval.
    #[must_use]
    pub fn label(self, words: &'static Translator) -> &'static str {
        match self {
            Self::Day => words.usage().day(),
            Self::Week => words.usage().week(),
        }
    }
}

/// A request's entire visible scope, retained across the asynchronous boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerSelection {
    /// Already parsed node-book key.
    pub machine: ConnectionContext,
    /// Rolling period; no session-lifetime interval can cross this boundary.
    pub range: Range,
}
