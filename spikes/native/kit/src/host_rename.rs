//! The reference's small static rename action, separate from labelled MD.
//!
//! [Decision Log]
//! - Purpose: one host-name edit control for every machine detail consumer.
//! - Constraint: source has no responsive size or hover/pressed palette.
//! - Alternatives: force a labelled Ghost button's MD geometry/85% press,
//!   or draw another area-owned pencil and hit target.
//! - Choice: an original kit Pencil at `icon::SM` on a static frame of the
//!   glyph and `spacing::S1` each side, radius `radius::MD`, and a hit area
//!   of that frame and `spacing::S2` each side; the frame and the hit are
//!   computed from those tokens, never restated. Disabled callers attach no
//!   handler, and the kit adds no opacity.
//! - Why: source `host-appearance-section.tsx` uses `ICON_SIZE.sm`,
//!   padding 4, `borderRadius.md` and `hitSlop={8}` in both layouts (a 22
//!   frame, a 38 touch hit: a source-specific exception to the MD button).
//! - Difference from the source: Paseo's pencil is a button named for
//!   assistive technology (`accessibilityRole="button"`, the edit label).
//!   This one has no accessible name and no keyboard reach: the kit has no
//!   accessibility tree and no focusable buttons yet, a kit-wide gap with no
//!   owner named. It takes no label until then, and shows no tooltip,
//!   because Paseo's pencil shows none.
//! - Cost: callback/pending state and rename editor remain caller-owned.

use crate::{
    ButtonState, Tokens,
    assets::Glyph,
    theme::{icon as icon_size, radius, spacing},
};
use gpui::prelude::*;
use gpui::{Div, ElementId, MouseButton, Pixels, Stateful, Svg, div, px, svg};

/// Paseo's `hitSlop={8}`: the hit reaches this far past the frame.
const SLOP: f32 = spacing::S2;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Geometry {
    visual: Pixels,
    hit: Pixels,
    glyph: Pixels,
    padding: Pixels,
    radius: Pixels,
}

/// The frame is the glyph and its padding; the hit is the frame and the
/// slop, both on each side.
fn geometry() -> Geometry {
    let visual = spacing::S1.mul_add(2.0, icon_size::SM);
    Geometry {
        visual: px(visual),
        hit: px(SLOP.mul_add(2.0, visual)),
        glyph: px(icon_size::SM),
        padding: px(spacing::S1),
        radius: px(radius::MD),
    }
}

/// Static Pencil action; the caller wires the editor only when enabled.
/// An enabled action consumes left-button down and release in bubble phase;
/// the generated caller click runs before release is stopped.
#[must_use]
pub fn host_rename_button(
    id: impl Into<ElementId>,
    state: ButtonState,
    tokens: &Tokens,
) -> Stateful<Div> {
    let geometry = geometry();
    let visual = div()
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(geometry.visual)
        .p(geometry.padding)
        .rounded(geometry.radius)
        .child(icon(geometry.glyph, tokens));
    div()
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(geometry.hit)
        .m(px(SLOP.copysign(-1.0)))
        .p(px(SLOP))
        .when(state == ButtonState::Enabled, |button| {
            button
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
                // GPUI's reverse bubble order runs the generated caller click
                // before this custom release listener. Capture must stay open
                // so pending press state is cleared even when bubble stops.
                .on_mouse_up(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
        })
        .child(visual)
}
fn icon(size: Pixels, tokens: &Tokens) -> Svg {
    svg()
        .path(Glyph::Pencil.path())
        .size(size)
        .text_color(tokens.neutral.foreground_muted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scheme;

    /// The values are the tokens', and the frame and hit follow from them:
    /// a padding or slop change moves both, never one of them.
    #[test]
    fn rename_is_static_and_does_not_use_labelled_md_geometry() {
        let geometry = geometry();
        assert_eq!(geometry.glyph, px(icon_size::SM));
        assert_eq!(geometry.padding, px(spacing::S1));
        assert_eq!(geometry.radius, px(radius::MD));
        let visual = spacing::S1.mul_add(2.0, icon_size::SM);
        assert_eq!(geometry.visual, px(visual));
        assert_eq!(geometry.hit, px(SLOP.mul_add(2.0, visual)));
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tokens = Tokens::of(scheme);
            let mut vector = icon(geometry.glyph, &tokens);
            assert_eq!(
                vector.style().text.color,
                Some(tokens.neutral.foreground_muted)
            );
            for state in [ButtonState::Enabled, ButtonState::Disabled] {
                let mut button = host_rename_button("rename", state, &tokens);
                let style = button.style();
                assert_eq!(style.size.width, Some(geometry.hit.into()));
                assert_eq!(style.size.height, Some(geometry.hit.into()));
                assert_eq!(style.opacity, None);
            }
        }
    }
}

#[cfg(test)]
mod events;
