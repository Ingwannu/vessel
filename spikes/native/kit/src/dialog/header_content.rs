//! Additive title-row decoration leaves every legacy content literal intact.
//!
//! A new public struct field, even optional, would break existing Rust literals.
//! The renderer instead accepts this typed input through From; plain content
//! carries no actions, while the builder retains one caller-owned UI element.
//! Its callbacks remain the caller's domain authority, not dialog admission.

use super::DialogContent;
use gpui::{AnyElement, IntoElement};

/// Content plus an optional trailing title-row action element, on one renderer.
pub struct DialogInput {
    content: DialogContent,
    header_actions: Option<AnyElement>,
}

impl DialogContent {
    /// Place caller-owned controls before the existing close button. Callbacks
    /// must retain and check their own current context/ticket before acting.
    #[must_use]
    pub fn with_header_actions(self, actions: impl IntoElement) -> DialogInput {
        DialogInput {
            content: self,
            header_actions: Some(actions.into_any_element()),
        }
    }
}

impl From<DialogContent> for DialogInput {
    fn from(content: DialogContent) -> Self {
        Self {
            content,
            header_actions: None,
        }
    }
}

impl DialogInput {
    pub(super) fn parts(self) -> (DialogContent, Option<AnyElement>) {
        (self.content, self.header_actions)
    }
}

impl std::fmt::Debug for DialogInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogInput")
            .field("content", &self.content)
            .field("header_actions", &self.header_actions.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
