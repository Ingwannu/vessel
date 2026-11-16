//! Paseo's rename-modal field profile over the existing bare text renderer.
//!
//! [Decision Log]
//! - Purpose: surface0/padding12/radius6/border1 differs from the generic
//!   form's surface2 and 32-or-44 shell.
//! - Choice: wrap `TextField::bare` and the shared inset-focus bridge,
//!   preserving the same engine, selection, focus and weak IME registry.
//! - Constraint: static neutral border, no focus ring, density branch or
//!   extra disabled paint. The shared `Editability` rejects pending edits.
//! - Geometry: retain native Body14/20 with the source's insets.

use gpui::prelude::*;
use gpui::{App, Div, px};

use super::{TextField, settings_frame};
use crate::theme::{Tokens, radius, spacing};

impl TextField {
    /// Rename's profile around the same bare editor. The shared frame lets
    /// padding focus it without reselecting a caret placed by the person.
    #[must_use]
    pub fn rename_element(&self, tokens: &Tokens, cx: &mut App) -> Div {
        settings_frame(self)
            .w_full()
            .p(px(spacing::S3))
            .rounded(px(radius::MD))
            .bg(tokens.neutral.surface0)
            .border_1()
            .border_color(tokens.neutral.border)
            .child(self.bare(tokens, cx))
    }
}
