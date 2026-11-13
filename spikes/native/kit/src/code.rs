//! The colours code is drawn in: the syntax tokens and a diff's added and
//! removed lines (`web/src/ui/tokens.css`, `--syn-*` and `--diff-*`), by
//! the page's values in both themes. A set of its own beside [`Tokens`]:
//! only the code panel and the setup log use them.
//!
//! [`Tokens`]: crate::Tokens

use gpui::{Hsla, rgb, rgba};

use crate::theme::Scheme;

/// The code colours of one scheme.
#[derive(Debug, Clone, Copy)]
pub struct CodeTokens {
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
        match scheme {
            Scheme::Light => Self {
                keyword: rgb(0x007a_3db3).into(),
                string: rgb(0x003b_6d11).into(),
                function: rgb(0x001d_4ed8).into(),
                number: rgb(0x00b5_471c).into(),
                comment: rgb(0x0066_6666).into(),
                constant: rgb(0x008a_5a00).into(),
                error: rgb(0x009d_0006).into(),
                // rgb(21 128 61 / 8%), rgb(157 0 6 / 8%).
                added_bg: rgba(0x1580_3d14).into(),
                removed_bg: rgba(0x9d00_0614).into(),
            },
            Scheme::Dark => Self {
                keyword: rgb(0x00c7_92ea).into(),
                string: rgb(0x00c3_e88d).into(),
                function: rgb(0x0093_b4ff).into(),
                number: rgb(0x00f7_8c6c).into(),
                comment: rgb(0x008f_8f8f).into(),
                constant: rgb(0x00ff_cb6b).into(),
                error: rgb(0x00fb_4934).into(),
                // rgb(74 222 128 / 10%), rgb(251 73 52 / 10%).
                added_bg: rgba(0x4ade_801a).into(),
                removed_bg: rgba(0xfb49_341a).into(),
            },
        }
    }
}
