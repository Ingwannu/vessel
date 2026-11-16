//! Additive title-row decoration leaves every legacy content literal intact.
//!
//! A new public struct field, even optional, would break existing Rust literals.
//! The renderer instead accepts this typed input through From; plain content
//! carries no actions, while the builder retains one caller-owned UI element.
//! Its callbacks remain the caller's domain authority, not dialog admission.

use super::DialogContent;
use super::header_search::HeaderSearch;
use gpui::{AnyElement, IntoElement};

/// Content plus an optional trailing title-row action element and an optional
/// header search row, on one renderer.
pub struct DialogInput {
    content: DialogContent,
    header_actions: Option<AnyElement>,
    header_search: Option<HeaderSearch>,
}

impl DialogContent {
    /// Place caller-owned controls before the existing close button. Callbacks
    /// must retain and check their own current context/ticket before acting.
    #[must_use]
    pub fn with_header_actions(self, actions: impl IntoElement) -> DialogInput {
        DialogInput {
            content: self,
            header_actions: Some(actions.into_any_element()),
            header_search: None,
        }
    }

    /// Put the owner's search under the title, inside the header's rule.
    #[must_use]
    pub fn with_header_search(self, search: HeaderSearch) -> DialogInput {
        DialogInput::from(self).with_header_search(search)
    }
}

impl From<DialogContent> for DialogInput {
    fn from(content: DialogContent) -> Self {
        Self {
            content,
            header_actions: None,
            header_search: None,
        }
    }
}

/// What the header draws beside and under the title.
pub(super) struct HeaderParts {
    pub(super) actions: Option<AnyElement>,
    pub(super) search: Option<HeaderSearch>,
}

impl DialogInput {
    /// Paseo's search row under the title; the owner keeps the query.
    #[must_use]
    pub fn with_header_search(mut self, search: HeaderSearch) -> Self {
        self.header_search = Some(search);
        self
    }

    pub(super) fn parts(self) -> (DialogContent, HeaderParts) {
        (
            self.content,
            HeaderParts {
                actions: self.header_actions,
                search: self.header_search,
            },
        )
    }
}

impl std::fmt::Debug for DialogInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogInput")
            .field("content", &self.content)
            .field("header_actions", &self.header_actions.is_some())
            .field("header_search", &self.header_search.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
