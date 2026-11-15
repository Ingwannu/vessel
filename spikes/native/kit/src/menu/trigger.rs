//! One action-trigger frame: source-specific glyph/paint/geometry and passive slots.
//!
//! [Decision Log]
//! - Purpose: `ProviderRow` and Git's `Some`/`None` header reuse the existing trigger.
//! - Constraints: Schedule/Git defaults, hit extents and inner anchor stay intact.
//! - Choice: one profile frame, explicit open presentation, and a read-only visual
//!   extent; a passive slot has no identity, glyph, anchor or input registration.
//! - Alternatives: an area icon button, three trigger renderers, an invisible
//!   trigger, or copied26/32 constants beside every Git count.
//! - Why/cost: source geometry is one fact; consumers still own target/authority
//!   guards and actual first-use hit/clip/focus proof. No disabled opacity is added.
use super::TriggerAnchor;
use crate::{
    assets::Glyph,
    picker::anchor_probe,
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
        TriggerSizing::Touch => crate::control_sizing::TOUCH_TARGET,
    };
    Geometry {
        minimum: match sizing {
            TriggerSizing::Pointer => px(SCHEDULE_MINIMUM),
            TriggerSizing::Touch => px(crate::control_sizing::TOUCH_TARGET),
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

/// The reference surface that owns this trigger's visual geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerProfile {
    /// Schedule row: 14-point glyph, radius 4, padding 4, hover wash.
    Schedule,
    /// Git header overflow: 16-point glyph, 26/32 visual square, radius 6.
    GitHeader,
    /// Provider row: horizontal14,32 visual in both layouts,radius8.
    ProviderRow,
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
    trigger_presented(
        id,
        anchor,
        profile,
        sizing,
        TriggerPresentation::Closed(state),
        tokens,
    )
}

/// Open wash/ink is an explicit caller state, separate from input availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerPresentation {
    /// Default for the existing entries.
    Closed(crate::ButtonState),
    /// Provider menu is open; Schedule/Git retain their source presentation.
    Open(crate::ButtonState),
}
impl TriggerPresentation {
    const fn availability(self) -> crate::ButtonState {
        match self {
            Self::Closed(state) | Self::Open(state) => state,
        }
    }
    const fn is_open(self) -> bool {
        matches!(self, Self::Open(_))
    }
}
const PROVIDER_VISUAL: Pixels = px(spacing::S8);
const PROVIDER_RADIUS: Pixels = px(crate::theme::radius::R2);

struct Frame {
    visual: Pixels,
    hit: Pixels,
    padding: Pixels,
    glyph: Pixels,
    asset: Glyph,
    radius: Pixels,
    minimum: Option<Pixels>,
}
fn frame(profile: TriggerProfile, sizing: TriggerSizing) -> Frame {
    match profile {
        TriggerProfile::Schedule => {
            let g = geometry(sizing);
            Frame {
                visual: g.visual,
                hit: g.hit,
                padding: g.padding,
                glyph: g.glyph,
                asset: Glyph::MoreVertical,
                radius: g.radius,
                minimum: Some(g.minimum),
            }
        }
        TriggerProfile::GitHeader => {
            let g = git_geometry(sizing);
            Frame {
                visual: g.visual,
                hit: g.hit,
                padding: px(0.0),
                glyph: g.glyph,
                asset: Glyph::MoreVertical,
                radius: g.radius,
                minimum: None,
            }
        }
        TriggerProfile::ProviderRow => Frame {
            visual: PROVIDER_VISUAL,
            hit: px(HIT_SLOP.mul_add(2.0, f32::from(PROVIDER_VISUAL))),
            padding: px(0.0),
            glyph: px(SCHEDULE_GLYPH),
            asset: Glyph::More,
            radius: PROVIDER_RADIUS,
            minimum: None,
        },
    }
}
/// Layout footprint from the exact geometry the active trigger draws.
#[must_use]
pub fn trigger_visual_size(profile: TriggerProfile, sizing: TriggerSizing) -> Pixels {
    frame(profile, sizing).visual
}
/// Passive `Some`/`None` slot: size only, with no identity, hit region or callbacks.
#[must_use]
pub fn trigger_space(profile: TriggerProfile, sizing: TriggerSizing) -> Div {
    div().flex_none().size(trigger_visual_size(profile, sizing))
}
fn profile_paint(
    profile: TriggerProfile,
    presentation: TriggerPresentation,
    hovered: bool,
    tokens: &Tokens,
) -> Paint {
    match profile {
        TriggerProfile::Schedule => paint(tokens, hovered),
        TriggerProfile::GitHeader => paint(tokens, false),
        TriggerProfile::ProviderRow => paint(tokens, hovered || presentation.is_open()),
    }
}
fn pressed_fill(profile: TriggerProfile, tokens: &Tokens) -> Option<Hsla> {
    match profile {
        TriggerProfile::ProviderRow => Some(tokens.neutral.surface3),
        TriggerProfile::Schedule | TriggerProfile::GitHeader => None,
    }
}
/// One visual/hit/anchor renderer for each reference profile, including open state.
/// Provider pressed changes background only; touch press alone does not invent
/// foreground icon ink. Disabled is not given an opacity absent from the source.
#[must_use]
pub fn trigger_presented(
    id: impl Into<ElementId>,
    anchor: &TriggerAnchor,
    profile: TriggerProfile,
    sizing: TriggerSizing,
    presentation: TriggerPresentation,
    tokens: &Tokens,
) -> Stateful<Div> {
    let id = id.into();
    let g = frame(profile, sizing);
    let enabled = presentation.availability() == crate::ButtonState::Enabled;
    let visual = visual(&id, &g, profile, sizing, presentation, anchor, tokens);
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .when(profile != TriggerProfile::GitHeader, |element| {
            element.group(TRIGGER_GROUP)
        })
        .size(g.hit)
        .m(px(HIT_SLOP.copysign(-1.0)))
        .p(px(HIT_SLOP))
        .when(profile == TriggerProfile::GitHeader, |element| {
            element.text_color(tokens.neutral.foreground_muted)
        })
        .when(enabled, |element| {
            element
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
        })
        .child(visual)
}
fn visual(
    id: &ElementId,
    g: &Frame,
    profile: TriggerProfile,
    sizing: TriggerSizing,
    presentation: TriggerPresentation,
    anchor: &TriggerAnchor,
    tokens: &Tokens,
) -> gpui::AnyElement {
    let idle = profile_paint(profile, presentation, false, tokens);
    let hover = profile_paint(profile, presentation, true, tokens);
    let enabled = presentation.availability() == crate::ButtonState::Enabled;
    let vector = match presentation {
        TriggerPresentation::Closed(state) => icon(profile, sizing, state, tokens),
        TriggerPresentation::Open(_) => presented_icon(profile, sizing, presentation, tokens),
    };
    let visual = div()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .size(g.visual)
        .p(g.padding)
        .rounded(g.radius)
        .when_some(g.minimum, |element, minimum| {
            element.min_w(minimum).min_h(minimum)
        })
        .when(profile != TriggerProfile::GitHeader, |element| {
            element.bg(idle.fill)
        })
        .when(enabled && profile != TriggerProfile::GitHeader, |element| {
            element.group_hover(TRIGGER_GROUP, move |style| style.bg(hover.fill))
        })
        .child(anchor_probe(anchor))
        .child(vector);
    match profile {
        TriggerProfile::ProviderRow => visual
            .id(ElementId::NamedChild(
                std::sync::Arc::new(id.clone()),
                "visual".into(),
            ))
            .when(enabled, |element| {
                // group_active requires a stateful element on pinned GPUI;
                // it refines after group_hover and paints only this inner box.
                element.when_some(pressed_fill(profile, tokens), |element, fill| {
                    element.group_active(TRIGGER_GROUP, move |style| style.bg(fill))
                })
            })
            .into_any_element(),
        TriggerProfile::Schedule | TriggerProfile::GitHeader => visual.into_any_element(),
    }
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
    presented_icon(profile, sizing, TriggerPresentation::Closed(state), tokens)
}
fn presented_icon(
    profile: TriggerProfile,
    sizing: TriggerSizing,
    presentation: TriggerPresentation,
    tokens: &Tokens,
) -> Svg {
    let g = frame(profile, sizing);
    let size = g.glyph;
    let idle = profile_paint(profile, presentation, false, tokens);
    let hover = profile_paint(profile, presentation, true, tokens);
    svg()
        .path(g.asset.path())
        .size(size)
        .flex_shrink_0()
        .text_color(idle.ink)
        .when(
            profile != TriggerProfile::GitHeader
                && presentation.availability() == crate::ButtonState::Enabled,
            |icon| icon.group_hover(TRIGGER_GROUP, move |style| style.text_color(hover.ink)),
        )
}

#[cfg(test)]
mod tests;
