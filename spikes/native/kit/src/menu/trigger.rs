//! Shared vertical action triggers, with reference-specific geometry.
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
    trigger_with(
        id,
        anchor,
        TriggerProfile::Schedule,
        sizing,
        crate::ButtonState::Enabled,
        tokens,
    )
}

fn schedule_trigger(
    id: impl Into<ElementId>,
    anchor: &TriggerAnchor,
    sizing: TriggerSizing,
    state: crate::ButtonState,
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
        .when(state == crate::ButtonState::Enabled, |element| {
            element
                .cursor_pointer()
                .hover(move |style| style.text_color(hover.ink).bg(hover.fill))
        })
        .child(anchor_probe(anchor))
        .child(
            svg()
                .path(Glyph::MoreVertical.path())
                .size(geometry.glyph)
                .flex_shrink_0(),
        )
}

/// The reference surface that owns this trigger's visual geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerProfile {
    /// Schedule row: 14-point glyph, radius 4, padding 4, hover wash.
    Schedule,
    /// Git header overflow: 16-point glyph, 26/32 visual square, radius 6.
    GitHeader,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GitGeometry {
    pub visual: Pixels,
    pub hit: Pixels,
    pub outset: Pixels,
    pub glyph: Pixels,
    pub radius: Pixels,
}
pub(super) const fn git_geometry(sizing: TriggerSizing) -> GitGeometry {
    let (visual, hit) = match sizing {
        TriggerSizing::Pointer => (26.0, 42.0),
        TriggerSizing::Touch => (32.0, 48.0),
    };
    GitGeometry {
        visual: px(visual),
        hit: px(hit),
        outset: px(spacing::S2),
        glyph: px(16.0),
        radius: px(6.0),
    }
}

/// Choose the reference profile without changing the schedule entry.
/// Disabled callers attach no handler, as for the kit's buttons. The git
/// reference supplies no disabled opacity or hover/pressed/open override.
#[must_use]
pub fn trigger_with(
    id: impl Into<ElementId>,
    anchor: &TriggerAnchor,
    profile: TriggerProfile,
    sizing: TriggerSizing,
    state: crate::ButtonState,
    tokens: &Tokens,
) -> Stateful<Div> {
    match profile {
        TriggerProfile::Schedule => schedule_trigger(id, anchor, sizing, state, tokens),
        TriggerProfile::GitHeader => git_trigger(id, anchor, sizing, state, tokens),
    }
}

fn git_trigger(
    id: impl Into<ElementId>,
    anchor: &TriggerAnchor,
    sizing: TriggerSizing,
    state: crate::ButtonState,
    tokens: &Tokens,
) -> Stateful<Div> {
    let geometry = git_geometry(sizing);
    // GPUI has no Pressable hitSlop. Outset the interactive box while negative
    // margins retain the 26/32 layout footprint. Anchor the menu to the inner
    // visual frame, never to this expanded hit box. The caller's separate
    // marginRight=-8 belongs on a wrapper so these four margins survive.
    div()
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(geometry.hit)
        .m(px(spacing::S2.copysign(-1.0)))
        .p(geometry.outset)
        .text_color(tokens.neutral.foreground_muted)
        .when(state == crate::ButtonState::Enabled, Styled::cursor_pointer)
        // Paseo's overflow style is static: schedule's hover wash/ink callback
        // must not be inherited by this profile.
        .child(
            div()
                .relative()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(geometry.visual)
                .rounded(geometry.radius)
                .child(anchor_probe(anchor))
                .child(svg().path(Glyph::MoreVertical.path()).size(geometry.glyph)),
        )
}
