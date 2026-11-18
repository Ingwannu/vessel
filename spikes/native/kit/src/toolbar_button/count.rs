//! The open-tab count drawn in a toolbar button instead of a glyph.
//!
//! [Decision Log]
//! - Purpose: TRACKER 789. Paseo's compact tab switcher trigger
//!   (`compact-workspace-header/tab-switcher.tsx`, `tabCountBadge`) is the
//!   header's icon chrome holding the number of open tabs, not a glyph.
//! - Values (Paseo's): at least 16 wide and 16 tall, 2 across, radius `sm`
//!   (2), a 1-point border of `foregroundMuted`; the number 10 on a 12 line,
//!   medium, in `foregroundMuted`. The comment there: sized and stroked like
//!   the 16 glyphs beside it.
//! - Alternatives: an app-drawn square beside the toolbar buttons (a second
//!   button style); a glyph per number (there are none).
//! - Choice: one constructor beside `loading`, through the same frame, so its
//!   hitbox, hover, press, disabled guard and tooltip are the toolbar's.
//! - Cost: no tabular figures (GPUI's text style has no font features here);
//!   a count of three digits widens the badge, as Paseo's `minWidth` does.

use crate::Tokens;
use crate::fonts::Type;
use crate::widgets::TypeExt as _;
use gpui::prelude::*;
use gpui::{Div, FontWeight, div, px};

#[cfg(test)]
mod tests;

/// Paseo's `tabCountBadge`: its least width and its height.
const SIDE: f32 = 16.0;
/// Paseo's `paddingHorizontal: 2`.
const ACROSS: f32 = 2.0;
/// Paseo's `borderRadius.sm`.
const RADIUS: f32 = 2.0;
/// Paseo's `tabCountText`: 10 on a 12 line.
const TEXT: f32 = 10.0;
const LINE: f32 = 12.0;

/// `count` in Paseo's outlined badge, muted.
pub(super) fn badge(count: usize, tokens: &Tokens) -> Div {
    let ink = tokens.neutral.foreground_muted;
    let badge = div();
    #[cfg(test)]
    let badge = badge.debug_selector(|| "toolbar-count-badge".to_owned());
    badge
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .min_w(px(SIDE))
        .h(px(SIDE))
        .px(px(ACROSS))
        .rounded(px(RADIUS))
        .border_1()
        .border_color(ink)
        .type_(Type::Kicker)
        .text_size(px(TEXT))
        .line_height(px(LINE))
        .font_weight(FontWeight::MEDIUM)
        .text_color(ink)
        .child(count.to_string())
}
