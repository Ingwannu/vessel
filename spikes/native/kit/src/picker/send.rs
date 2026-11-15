//! The composer's send button (split from `picker.rs`, which holds the
//! pills and the combobox: the file budget, docs/QUALITY.md M3).

use gpui::prelude::*;
use gpui::{Div, ElementId, Stateful, div, px, rgb};

use super::glyph;
use crate::assets::Glyph;
use crate::theme::Tokens;

/// What the round button beside the composer does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendKind {
    /// Send the text (an arrow up, on the accent).
    Send,
    /// Stop the running turn (a square, on red).
    Stop,
}

/// Paseo's send button: a circle 28 across (44 on touch), the accent with
/// a white arrow, half-faded while there is nothing to send; while a turn
/// runs and nothing is typed, a red circle with a white square.
#[must_use]
pub fn send_button(
    id: impl Into<ElementId>,
    kind: SendKind,
    enabled: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    // Paseo's stop is `palette.red[600]`, the same in both themes.
    let (fill, mark) = match kind {
        SendKind::Send => (neutral.accent, Glyph::ArrowUp),
        SendKind::Stop => (rgb(0x00dc_2626).into(), Glyph::Stop),
    };
    let size = if touch { 44.0 } else { 28.0 };
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .size(px(size))
        .rounded_full()
        .bg(fill)
        .when(!enabled, |button| button.opacity(0.5))
        .when(enabled, Styled::cursor_pointer)
        .child(glyph(
            mark,
            if touch { 20.0 } else { 16.0 },
            neutral.accent_foreground,
        ))
}
