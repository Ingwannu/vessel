//! The colours code is drawn in: the syntax tokens and a diff's added and
//! removed lines (`web/src/ui/tokens.css`, `--syn-*` and `--diff-*`), by
//! scheme defaults for error/diff tints. Syntax ink comes from the selected
//! independent catalogue; no second syntax table lives beside [`Tokens`].
//!
//! [`Tokens`]: crate::Tokens

use gpui::{Hsla, rgb, rgba};

use crate::syntax::SyntaxStyle;
use crate::theme::Scheme;

/// The code colours of one scheme.
#[derive(Debug, Clone, Copy)]
pub struct CodeTokens {
    /// Plain variables/punctuation from the independent syntax palette.
    pub plain: Hsla,
    /// Markup tags, separate from a language keyword.
    pub tag: Hsla,
    /// `--syn-keyword` (and markup's tags, as the page draws them).
    pub keyword: Hsla,
    /// `--syn-string`.
    pub string: Hsla,
    /// `--syn-func`.
    pub function: Hsla,
    /// `--syn-number`.
    pub number: Hsla,
    /// `--syn-comment`.
    pub comment: Hsla,
    /// `--syn-const`.
    pub constant: Hsla,
    /// `--syn-error`.
    pub error: Hsla,
    /// `--diff-add-bg`.
    pub added_bg: Hsla,
    /// `--diff-del-bg`.
    pub removed_bg: Hsla,
}

impl CodeTokens {
    /// The code colours of `scheme`.
    #[must_use]
    pub fn of(scheme: Scheme) -> Self {
        let colors = crate::syntax::selected().colours(scheme);
        let (error, added_bg, removed_bg) = match scheme {
            Scheme::Light => (
                rgb(0x009d_0006).into(),
                rgba(0x1580_3d14).into(),
                rgba(0x9d00_0614).into(),
            ),
            Scheme::Dark => (
                rgb(0x00fb_4934).into(),
                rgba(0x4ade_801a).into(),
                rgba(0xfb49_341a).into(),
            ),
        };
        Self {
            plain: colors.colour(SyntaxStyle::Variable),
            tag: colors.colour(SyntaxStyle::Tag),
            keyword: colors.colour(SyntaxStyle::Keyword),
            string: colors.colour(SyntaxStyle::String),
            function: colors.colour(SyntaxStyle::Function),
            number: colors.colour(SyntaxStyle::Number),
            comment: colors.colour(SyntaxStyle::Comment),
            constant: colors.colour(SyntaxStyle::Constant),
            error,
            added_bg,
            removed_bg,
        }
    }
}
