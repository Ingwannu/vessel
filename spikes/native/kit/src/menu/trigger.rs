//! The schedule kebab's own hit area, not a labelled pill.
use super::TriggerAnchor;
use crate::{
    assets::Glyph,
    picker::anchor_probe,
    popup,
    theme::{Tokens, spacing},
};
use gpui::prelude::*;
use gpui::{Div, ElementId, Hsla, Pixels, Stateful, div, px, svg};

/// The reference's pointer/touch geometry gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerSizing {
    /// Pointer minimum20; the 14 glyph and 4 padding produce a 22-point hit box.
    Pointer,
    /// One 44-point touch target.
    Touch,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Geometry {
    pub minimum: Pixels,
    pub padding: Pixels,
    pub glyph: Pixels,
    pub radius: Pixels,
}
pub(super) const fn geometry(sizing: TriggerSizing) -> Geometry {
    Geometry {
        minimum: match sizing {
            TriggerSizing::Pointer => px(20.0),
            TriggerSizing::Touch => px(popup::TOUCH_TARGET),
        },
        padding: px(spacing::S1),
        glyph: px(14.0),
        radius: px(spacing::S1),
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Paint {
    pub ink: Hsla,
    pub fill: Hsla,
}
pub(super) fn paint(tokens: &Tokens, hovered: bool) -> Paint {
    let n = tokens.neutral;
    Paint {
        ink: if hovered {
            n.foreground
        } else {
            n.foreground_muted
        },
        fill: if hovered {
            n.surface2
        } else {
            gpui::transparent_black()
        },
    }
}
/// Vertical dots at14, radius4 and padding4, after `schedule-row.tsx`.
/// The captured bounds let `MenuState::open_end` align the surface with this
/// trigger, regardless of where inside its hit area the pointer was pressed.
#[must_use]
pub fn trigger(
    id: impl Into<ElementId>,
    anchor: &TriggerAnchor,
    sizing: TriggerSizing,
    tokens: &Tokens,
) -> Stateful<Div> {
    let geometry = geometry(sizing);
    let idle = paint(tokens, false);
    let hover = paint(tokens, true);
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .min_w(geometry.minimum)
        .min_h(geometry.minimum)
        .p(geometry.padding)
        .rounded(geometry.radius)
        .text_color(idle.ink)
        .bg(idle.fill)
        .cursor_pointer()
        .hover(move |style| style.text_color(hover.ink).bg(hover.fill))
        .child(anchor_probe(anchor))
        .child(
            svg()
                .path(Glyph::MoreVertical.path())
                .size(geometry.glyph)
                .flex_shrink_0(),
        )
}
