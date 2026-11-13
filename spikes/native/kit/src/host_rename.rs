//! The reference's small static rename action, separate from labelled MD.
//!
//! [Decision Log]
//! - Purpose: one host-name edit control for every machine detail consumer.
//! - Constraint: source has no responsive size or hover/pressed palette.
//! - Alternatives: force a labelled Ghost button's MD geometry/85% press,
//!   or draw another area-owned pencil and hit target.
//! - Choice: an original kit Pencil14 on a static22 frame/hit38 shell;
//!   disabled callers attach no handler, and the kit adds no opacity.
//! - Why: source `host-appearance-section.tsx` uses padding4/radius6 and
//!   hitSlop8 in both layouts. The38 touch hit is a source-specific exception.
//! - Cost: callback/pending state and rename editor remain caller-owned.

use crate::{ButtonState, Tokens, assets::Glyph, theme::spacing};
use gpui::prelude::*;
use gpui::{Div, ElementId, MouseButton, Pixels, Stateful, Svg, div, px, svg};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Geometry {
    visual: Pixels,
    hit: Pixels,
    glyph: Pixels,
    padding: Pixels,
    radius: Pixels,
}
const GEOMETRY: Geometry = Geometry {
    visual: px(22.0),
    hit: px(38.0),
    glyph: px(14.0),
    padding: px(spacing::S1),
    radius: px(6.0),
};

/// Static Pencil action; the caller wires the editor only when enabled.
#[must_use]
pub fn host_rename_button(
    id: impl Into<ElementId>,
    state: ButtonState,
    tokens: &Tokens,
) -> Stateful<Div> {
    let visual = div()
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(GEOMETRY.visual)
        .p(GEOMETRY.padding)
        .rounded(GEOMETRY.radius)
        .child(icon(tokens));
    div()
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(GEOMETRY.hit)
        .m(px(spacing::S2.copysign(-1.0)))
        .p(px(spacing::S2))
        .when(state == ButtonState::Enabled, |button| {
            button
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
        })
        .child(visual)
}
fn icon(tokens: &Tokens) -> Svg {
    svg()
        .path(Glyph::Pencil.path())
        .size(GEOMETRY.glyph)
        .text_color(tokens.neutral.foreground_muted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scheme;
    #[test]
    fn rename_is_static_and_does_not_use_labelled_md_geometry() {
        assert_eq!(
            GEOMETRY,
            Geometry {
                visual: px(22.0),
                hit: px(38.0),
                glyph: px(14.0),
                padding: px(4.0),
                radius: px(6.0)
            }
        );
        for (scheme, muted) in [(Scheme::Light, 0x0071_717a), (Scheme::Dark, 0x00a1_a5a4)] {
            let tokens = Tokens::of(scheme);
            let mut vector = icon(&tokens);
            assert_eq!(vector.style().text.color, Some(gpui::rgb(muted).into()));
            for state in [ButtonState::Enabled, ButtonState::Disabled] {
                let mut button = host_rename_button("rename", state, &tokens);
                let style = button.style();
                assert_eq!(style.size.width, Some(px(38.0).into()));
                assert_eq!(style.size.height, Some(px(38.0).into()));
                assert_eq!(style.opacity, None);
            }
        }
    }
}
