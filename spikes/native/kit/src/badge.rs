//! A short status label, after Paseo's `components/ui/status-badge.tsx`.
//!
//! [Decision Log]
//! - Purpose: agent, device and schedule rows share one status signal.
//! - Constraints: status is an enum, never a caller-chosen colour; words
//!   must remain whole. A badge is a label, not a button or a focus target.
//! - Alternatives: lane-coloured text (the board's vocabulary), a chip
//!   per area (different surfaces for the same state), a truncated label.
//! - Choice: Paseo's four variants; full radius, a one-point border,
//!   12-point normal text on a 16-point line, six between an optional
//!   leading element and the text. Small: eight across, three vertically
//!   (24 tall including the border); extra-small: six across, one
//!   vertically (20 tall). Semantic fill/text use the kit's status tokens;
//!   muted uses surface3, border and foregroundMuted. No hover or ring.
//! - Cost: callers must choose a short localized label; explanations go
//!   underneath the row, not into a shrinking or ellipsized badge.

use crate::{Tokens, Type, TypeExt as _};
use gpui::{AnyElement, Div, Hsla, SharedString, div, prelude::*, px};

/// Paseo's closed set of semantic badge variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// A successful or available state.
    Success,
    /// Needs attention.
    Warning,
    /// An error or refused state.
    Error,
    /// Neutral information.
    Muted,
}

/// The reference's two paddings, with the same text size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// A row's normal badge, 24 points tall.
    Small,
    /// Beside a smaller line, 20 points tall.
    ExtraSmall,
}

/// Draw one whole, non-interactive status label. `leading` is optional
/// content such as a status dot; its colour should use the same tokens.
#[must_use]
pub fn badge(
    label: impl Into<SharedString>,
    variant: Variant,
    size: Size,
    leading: Option<AnyElement>,
    tokens: &Tokens,
) -> Div {
    let look = look(variant, tokens);
    let (horizontal, vertical) = padding(size);
    div()
        .flex()
        .items_center()
        .flex_shrink_0()
        .gap(px(6.0))
        .rounded_full()
        .border_1()
        .border_color(look.border)
        .bg(look.fill)
        .px(px(horizontal))
        .py(px(vertical))
        .type_(Type::Meta)
        .text_color(look.text)
        .whitespace_nowrap()
        .children(leading)
        .child(label.into())
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Look {
    fill: Hsla,
    border: Hsla,
    text: Hsla,
}

fn look(variant: Variant, tokens: &Tokens) -> Look {
    let status = tokens.status();
    let signal = match variant {
        Variant::Success => status.success,
        Variant::Warning => status.warning,
        Variant::Error => status.error,
        Variant::Muted => {
            return Look {
                fill: tokens.neutral.surface3,
                border: tokens.neutral.border,
                text: tokens.neutral.foreground_muted,
            };
        }
    };
    Look {
        fill: signal.tint,
        border: gpui::transparent_black(),
        text: signal.foreground,
    }
}

const fn padding(size: Size) -> (f32, f32) {
    match size {
        Size::Small => (8.0, 3.0),
        Size::ExtraSmall => (6.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scheme;

    #[test]
    fn every_theme_maps_the_four_reference_variants_to_their_own_tokens() {
        // Paseo's status-badge.browser.test.ts: neutral shell, a tint of
        // the text's own semantic colour, transparent semantic borders.
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tokens = Tokens::of(scheme);
            let muted = look(Variant::Muted, &tokens);
            assert_eq!(
                muted,
                Look {
                    fill: tokens.neutral.surface3,
                    border: tokens.neutral.border,
                    text: tokens.neutral.foreground_muted
                }
            );
            let status = tokens.status();
            for (variant, signal) in [
                (Variant::Success, status.success),
                (Variant::Warning, status.warning),
                (Variant::Error, status.error),
            ] {
                let mapped = look(variant, &tokens);
                assert_eq!(
                    mapped,
                    Look {
                        fill: signal.tint,
                        border: gpui::transparent_black(),
                        text: signal.foreground
                    }
                );
                let mut same_hue = mapped.text;
                same_hue.a = mapped.fill.a;
                assert_eq!(mapped.fill, same_hue);
                assert!(mapped.fill.a > 0.0 && mapped.fill.a < 0.5);
            }
        }
    }

    #[test]
    fn exact_reference_signal_values_and_padding_do_not_drift() {
        let light = Tokens::of(Scheme::Light).status();
        let dark = Tokens::of(Scheme::Dark).status();
        assert_eq!(light.success.foreground, gpui::rgb(0x003e_704a).into());
        assert_eq!(light.warning.foreground, gpui::rgb(0x007b_5d39).into());
        assert_eq!(light.error.foreground, gpui::rgb(0x009d_433b).into());
        assert_eq!(dark.success.foreground, gpui::rgb(0x006c_b17b).into());
        assert_eq!(dark.warning.foreground, gpui::rgb(0x00c0_9664).into());
        assert_eq!(dark.error.foreground, gpui::rgb(0x00d8_847b).into());
        assert_eq!(light.success.tint, gpui::rgba(0x3e70_4a1f).into());
        assert_eq!(dark.success.tint, gpui::rgba(0x6cb1_7b29).into());
        assert_eq!(padding(Size::Small), (8.0, 3.0));
        assert_eq!(padding(Size::ExtraSmall), (6.0, 1.0));
    }
}
