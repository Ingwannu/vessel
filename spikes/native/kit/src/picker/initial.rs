//! A project's initial on a tile (moved out of `picker.rs`, which may not
//! grow, when rows gained their role, TRACKER 808).

use gpui::prelude::*;
use gpui::{Div, SharedString, div, px};

use crate::theme::Tokens;

/// A project's mark as Paseo draws one without an icon of its own
/// (`ProjectIconView`'s fallback): the name's first letter, upper case,
/// on a 16 tile, 10 high and semibold, muted.
#[must_use]
pub fn initial_tile(initial: SharedString, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .size(px(16.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .bg(neutral.surface3)
        .text_size(px(10.0))
        .line_height(px(16.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(neutral.foreground_muted)
        .child(initial)
}

/// A name's initial for [`initial_tile`] (`?` for an empty name).
#[must_use]
pub fn initial_of(name: &str) -> SharedString {
    name.chars().next().map_or_else(
        || "?".into(),
        |first| first.to_uppercase().collect::<String>().into(),
    )
}
