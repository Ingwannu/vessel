//! A provider's mark inside the existing 56-point settings row frame.
//!
//! [Decision Log]
//! - Constraint: p16 plus the following row's hairline leaves 23 points
//!   for contents, while `OpenCode` needs 36 including its quiet zone.
//! - Choice: a 24/23-high layout slot, with the unaltered 36-high image held
//!   inside the row's existing padding. The frame's height/insets stay put.
//! - Why: a literal 36-high content child would enlarge the row to 68/69.
//! - Cost: all provider mark columns reserve 32 for aligned names; the
//!   following border needs a half-point paint correction, tested in GPUI.
use super::{Artwork, OPENCODE, Size, Space, render};
use crate::Tokens;
use gpui::prelude::FluentBuilder as _;
use gpui::{Div, ParentElement as _, Styled, div, px};

/// Which edge the shared settings row frame draws above its contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRowEdge {
    /// First row, without a top hairline.
    First,
    /// Following row, with the shared frame's one-point hairline.
    Following,
}
/// Layout adapter for `settings::row_with_content(...).min_h(56)` with
/// that control's unchanged 16-point insets. Other surfaces use
/// [`super::agent_mark`] and their own typed [`Space`]. The returned slot is
/// 32 × 24 in the first row and 32 × 23 after a hairline; `OpenCode`'s
/// real 32 × 36 draw box remains inside its row.
#[must_use]
pub fn settings_mark(
    art: Option<Artwork>,
    size: Size,
    edge: SettingsRowEdge,
    tokens: &Tokens,
) -> Div {
    let image = render(
        art,
        size,
        Space::Lockup,
        tokens,
        tokens.neutral.foreground,
        None,
    );
    let image = if art == Some(OPENCODE) {
        let top = match edge {
            SettingsRowEdge::First => -6.0,
            SettingsRowEdge::Following => -7.0,
        };
        image.absolute().left(px(0.0)).top(px(top))
    } else {
        image.when(
            art.is_some() && edge == SettingsRowEdge::Following,
            |image| image.relative().top(px(-0.5)),
        )
    };
    let height = match edge {
        SettingsRowEdge::First => 24.0,
        SettingsRowEdge::Following => 23.0,
    };
    div()
        .when(art.is_some(), Styled::self_start)
        .relative()
        .w(px(32.0))
        .h(px(height))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(image)
}

#[cfg(test)]
mod tests;
