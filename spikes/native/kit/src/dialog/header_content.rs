//! Additive title-row decoration leaves every legacy content literal intact.
//!
//! A new public struct field, even optional, would break existing Rust literals.
//! The renderer instead accepts this typed input through From; plain content
//! carries no actions, while the builder retains one caller-owned UI element.
//! Its callbacks remain the caller's domain authority, not dialog admission.

use super::DialogContent;
use super::header_back::HeaderBack;
use super::header_search::HeaderSearch;
use gpui::{AnyElement, IntoElement};

/// Content plus an optional leading way back, an optional trailing title-row
/// action element and an optional header search row, on one renderer.
pub struct DialogInput {
    content: DialogContent,
    header_back: Option<HeaderBack>,
    header_actions: Option<AnyElement>,
    header_search: Option<HeaderSearch>,
    header_subtitle: Option<AnyElement>,
    title_only: bool,
}

impl DialogContent {
    /// Place caller-owned controls before the existing close button. Callbacks
    /// must retain and check their own current context/ticket before acting.
    #[must_use]
    pub fn with_header_actions(self, actions: impl IntoElement) -> DialogInput {
        DialogInput {
            content: self,
            header_back: None,
            header_actions: Some(actions.into_any_element()),
            header_search: None,
            header_subtitle: None,
            title_only: false,
        }
    }

    /// Paseo's combobox sheet header: the title alone (`header_title_only`).
    #[must_use]
    pub fn with_title_only(self) -> DialogInput {
        DialogInput::from(self).with_title_only()
    }

    /// Put the owner's search under the title, inside the header's rule.
    #[must_use]
    pub fn with_header_search(self, search: HeaderSearch) -> DialogInput {
        DialogInput::from(self).with_header_search(search)
    }

    /// Lead the title with the owner's way back (a sheet's inner view).
    #[must_use]
    pub fn with_header_back(self, back: HeaderBack) -> DialogInput {
        DialogInput::from(self).with_header_back(back)
    }
}

impl From<DialogContent> for DialogInput {
    fn from(content: DialogContent) -> Self {
        Self {
            content,
            header_back: None,
            header_actions: None,
            header_search: None,
            header_subtitle: None,
            title_only: false,
        }
    }
}

/// What the header draws beside and under the title.
pub(super) struct HeaderParts {
    pub(super) back: Option<HeaderBack>,
    pub(super) actions: Option<AnyElement>,
    pub(super) search: Option<HeaderSearch>,
    pub(super) subtitle: Option<AnyElement>,
    pub(super) title_only: bool,
}

impl DialogInput {
    /// Paseo's search row under the title; the owner keeps the query.
    #[must_use]
    pub fn with_header_search(mut self, search: HeaderSearch) -> Self {
        self.header_search = Some(search);
        self
    }

    /// Paseo's header back arrow before the title; the owner keeps what
    /// going back means.
    #[must_use]
    pub fn with_header_back(mut self, back: HeaderBack) -> Self {
        self.header_back = Some(back);
        self
    }

    /// Paseo's combobox sheet header (`combobox.tsx` `bottomSheetHeader`): the
    /// title alone, with no close button and no rule; Escape, the scrim and
    /// the handle still close it. Any back, actions, search or subtitle given
    /// with it are not drawn.
    #[must_use]
    pub const fn with_title_only(mut self) -> Self {
        self.title_only = true;
        self
    }

    /// Paseo's `SheetHeader.subtitle`: the owner's line under the title, in
    /// the title group (gap 4), such as the import sheet's scope.
    #[must_use]
    pub fn with_header_subtitle(mut self, subtitle: impl IntoElement) -> Self {
        self.header_subtitle = Some(subtitle.into_any_element());
        self
    }

    pub(super) fn parts(self) -> (DialogContent, HeaderParts) {
        (
            self.content,
            HeaderParts {
                back: self.header_back,
                actions: self.header_actions,
                search: self.header_search,
                subtitle: self.header_subtitle,
                title_only: self.title_only,
            },
        )
    }
}

impl std::fmt::Debug for DialogInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogInput")
            .field("content", &self.content)
            .field("header_back", &self.header_back.is_some())
            .field("header_actions", &self.header_actions.is_some())
            .field("header_search", &self.header_search.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
