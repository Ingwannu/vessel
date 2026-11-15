//! Shared vertical action triggers, with reference-specific geometry.
use super::TriggerAnchor;
use crate::{
    assets::Glyph,
    picker::anchor_probe,
    popup,
    theme::{Tokens, spacing},
};
use gpui::prelude::*;
use gpui::{Div, ElementId, Hsla, MouseButton, Pixels, Stateful, Svg, div, px, svg};

/// Backwards-compatible name for the one shared input-sizing type.
pub use crate::control_sizing::ControlSizing as TriggerSizing;

// Source props, named once. Hit extents derive from the visual and hitSlop,
// so retuning either cannot leave the interactive box at an unrelated size.
const SCHEDULE_GLYPH: f32 = super::LeadingSize::Small.points();
const SCHEDULE_MINIMUM: f32 = 20.0;
const GIT_POINTER: f32 = 26.0;
const GIT_TOUCH: f32 = 32.0;
const GIT_GLYPH: f32 = super::LeadingSize::Regular.points();
const GIT_RADIUS: f32 = 6.0;
const HIT_SLOP: f32 = spacing::S2;
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Geometry {
    pub minimum: Pixels,
    pub visual: Pixels,
    pub hit: Pixels,
    pub padding: Pixels,
    pub glyph: Pixels,
    pub radius: Pixels,
}
pub(super) fn geometry(sizing: TriggerSizing) -> Geometry {
    let visual = match sizing {
        TriggerSizing::Pointer => spacing::S1.mul_add(2.0, SCHEDULE_GLYPH),
        TriggerSizing::Touch => popup::TOUCH_TARGET,
    };
    Geometry {
        minimum: match sizing {
            TriggerSizing::Pointer => px(SCHEDULE_MINIMUM),
            TriggerSizing::Touch => px(popup::TOUCH_TARGET),
        },
        visual: px(visual),
        hit: px(HIT_SLOP.mul_add(2.0, visual)),
        padding: px(spacing::S1),
        glyph: px(SCHEDULE_GLYPH),
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
    let visual = div()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .min_w(geometry.minimum)
        .min_h(geometry.minimum)
        .size(geometry.visual)
        .p(geometry.padding)
        .rounded(geometry.radius)
        .bg(idle.fill)
        .when(state == crate::ButtonState::Enabled, |visual| {
            visual.group_hover(TRIGGER_GROUP, move |style| style.bg(hover.fill))
        })
        .child(anchor_probe(anchor))
        .child(icon(TriggerProfile::Schedule, sizing, state, tokens));
    // Source hitSlop=8 expands input, never layout, paint or menu anchor.
    // Group hover paints only the inner22/44 frame, even when its hit area
    // reaches into a row's padding. The parent's content mask still clips it.
    div()
        .id(id)
        .group(TRIGGER_GROUP)
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .size(geometry.hit)
        .m(px(HIT_SLOP.copysign(-1.0)))
        .p(px(HIT_SLOP))
        .when(state == crate::ButtonState::Enabled, |element| {
            element
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
        })
        .child(visual)
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
pub(super) fn git_geometry(sizing: TriggerSizing) -> GitGeometry {
    let visual = match sizing {
        TriggerSizing::Pointer => GIT_POINTER,
        TriggerSizing::Touch => GIT_TOUCH,
    };
    GitGeometry {
        visual: px(visual),
        hit: px(HIT_SLOP.mul_add(2.0, visual)),
        outset: px(HIT_SLOP),
        glyph: px(GIT_GLYPH),
        radius: px(GIT_RADIUS),
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
        .m(px(HIT_SLOP.copysign(-1.0)))
        .p(geometry.outset)
        .text_color(tokens.neutral.foreground_muted)
        .when(state == crate::ButtonState::Enabled, |element| {
            element
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
        })
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
                .child(icon(TriggerProfile::GitHeader, sizing, state, tokens)),
        )
}

// A group is scoped to its nearest ancestor by GPUI's hitbox stack, so rows
// can share this name without changing another trigger's hover ink.
const TRIGGER_GROUP: &str = "schedule-menu-trigger";

pub(super) fn icon(
    profile: TriggerProfile,
    sizing: TriggerSizing,
    state: crate::ButtonState,
    tokens: &Tokens,
) -> Svg {
    let size = match profile {
        TriggerProfile::Schedule => geometry(sizing).glyph,
        TriggerProfile::GitHeader => git_geometry(sizing).glyph,
    };
    let idle = paint(tokens, false);
    let hover = paint(tokens, true);
    // This GPUI version paints SVG only when the SVG's own style has ink;
    // a parent's text colour is not inherited by Svg::paint. Keep it on the
    // actual vector and opt only the schedule profile into group-hover ink.
    svg()
        .path(Glyph::MoreVertical.path())
        .size(size)
        .flex_shrink_0()
        .text_color(idle.ink)
        .when(
            profile == TriggerProfile::Schedule && state == crate::ButtonState::Enabled,
            |icon| icon.group_hover(TRIGGER_GROUP, move |style| style.text_color(hover.ink)),
        )
}
