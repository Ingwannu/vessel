//! A passive dot and one complete status label, after GOAT's
//! `SideChatSurface.tsx` and `index.css` text-2xs (11px/16px, normal).
//!
//! [Decision Log]
//! - Purpose: a compact header needs GOAT's single status signal.
//! - Constraints: existing badges are capsules; adding a second badge or
//!   screen-specific colours would change the reference and duplicate a fact.
//! - Choice: reuse the kit's 6px dot, faces and semantic token structure,
//!   with 4px gap and the reference's separate text/dot roles. No background,
//!   border, hover, focus target, truncation or action belongs to this label.
//! - Cost: the host must leave room for its whole translated label; the
//!   view decides its status and words, never this passive control.

use gpui::{Div, FontWeight, SharedString, div, prelude::*, px};

use crate::theme::{InlineStatus, spacing};
use crate::{Tokens, Type, TypeExt as _, dot};

/// The closed set of inline semantic signals; no caller-chosen colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Starting or working.
    Active,
    /// Waiting on the person.
    Attention,
    /// At rest, including an interrupted turn.
    Idle,
    /// Failed.
    Error,
}

const TEXT_SIZE: f32 = 11.0;
const LINE_HEIGHT: f32 = 16.0;

/// Draw one dot with one whole, non-interactive status label. The host
/// supplies an already translated label and owns any accessible context.
#[must_use]
pub fn inline_status(label: impl Into<SharedString>, tone: Tone, tokens: &Tokens) -> Div {
    let palette = InlineStatus::of(tokens.scheme);
    let signal = match tone {
        Tone::Active => palette.active,
        Tone::Attention => palette.attention,
        Tone::Idle => palette.idle,
        Tone::Error => palette.error,
    };
    div()
        .flex()
        .items_center()
        .flex_shrink_0()
        .gap(px(spacing::S1))
        .type_(Type::Meta)
        .text_size(px(crate::fonts::ui(TEXT_SIZE)))
        .line_height(px(crate::fonts::ui(LINE_HEIGHT)))
        .font_weight(FontWeight::NORMAL)
        .text_color(signal.foreground)
        .whitespace_nowrap()
        .child(dot(signal.dot))
        .child(label.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_metrics_are_eleven_by_sixteen_with_four_between_dot_and_label() {
        assert_eq!(TEXT_SIZE.to_bits(), 11.0_f32.to_bits());
        assert_eq!(LINE_HEIGHT.to_bits(), 16.0_f32.to_bits());
        assert_eq!(spacing::S1.to_bits(), 4.0_f32.to_bits());
    }
}
