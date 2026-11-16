//! A segmented control: one of two to four choices, all in view, as
//! Paseo's app draws it (`components/ui/segmented-control.tsx` and
//! `control-geometry.ts`): no track of its own, segments 4 apart, each a
//! pill (fully rounded) 4 shorter than the control (28 inside 32, `sm` 24
//! inside 28), 12 across (`sm` 8), the label in the muted ink at 14 (`sm`
//! 12) and normal weight; the chosen one on the third surface in the full
//! ink, a hovered one on the second; a disabled one at half opacity.
//!
//! The page's control (a track on the field fill with a lifted thumb,
//! DESIGN.md 6.13) differs; the owner's rule makes Paseo's the look
//! (ADR 0036, the brief's "The look"), the page's the behaviour (one of a
//! few, all in view). Colours are Paseo's neutrals (`tokens.neutral`).
//!
//! The control draws; the caller owns the value and is told of a choice
//! through each segment's click, as the kit's other controls do.

//! Explicit `_at` entries additionally use MD44/inner40 on compact layouts and
//! the source unselected pressed surface3. Legacy entries retain pointer32/28
//! and their prior hover-only paint, so no existing caller changes silently.
//!
//! [Decision Log]
//! - Purpose: schedules need the pinned compact filter, not an area height override.
//! - Existing assets: one segmented renderer and shared `ControlSizing`/touch44.
//! - Alternatives: resize the caller's div, migrate all old owners, or duplicate it.
//! - Choice: additive typed layout entries, delegating to one private body; old
//!   functions keep pointer geometry and paint while new opt-ins get source press.
//! - Why/cost: every consumer can migrate deliberately; legacy callers remain
//!   staged pointer presentations until their owner opts in.

use gpui::prelude::*;
use gpui::{Div, ElementId, SharedString, Stateful, div, px};

use crate::control_sizing::{ControlSizing, TOUCH_TARGET};
use crate::fonts::Type;
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;

/// The control's size (Paseo's two control sizes, pointer-sized).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentSize {
    /// 32, the default.
    Medium,
    /// 28, for toolbars and nested rows.
    Small,
}

impl SegmentSize {
    /// The control's height.
    #[must_use]
    pub const fn height(self) -> f32 {
        match self {
            Self::Medium => 32.0,
            Self::Small => 28.0,
        }
    }

    /// A segment's height: 2 in from the control's edge on each side.
    #[must_use]
    pub const fn inner(self) -> f32 {
        match self {
            Self::Medium => 28.0,
            Self::Small => 24.0,
        }
    }
}

// `SEGMENTED_INSET` in pinned control-geometry.ts35/308–326: two sides.
const SEGMENT_INSET: f32 = spacing::S0_5;
#[derive(Clone, Copy)]
enum Presentation {
    Legacy,
    Source,
}
#[derive(Clone, Copy)]
struct Profile {
    size: SegmentSize,
    sizing: ControlSizing,
    presentation: Presentation,
}
struct Geometry {
    outer: f32,
    inner: f32,
    horizontal: f32,
    text: f32,
}
fn geometry(size: SegmentSize, sizing: ControlSizing) -> Geometry {
    let outer = match (size, sizing) {
        (SegmentSize::Medium, ControlSizing::Touch) => TOUCH_TARGET,
        (SegmentSize::Medium, ControlSizing::Pointer)
        | (SegmentSize::Small, ControlSizing::Pointer | ControlSizing::Touch) => size.height(),
    };
    let (horizontal, text) = match size {
        SegmentSize::Medium => (spacing::S3, 14.0),
        SegmentSize::Small => (spacing::S2, 12.0),
    };
    Geometry {
        outer,
        // Subtract the same source inset on both sides without unchecked arithmetic.
        inner: SEGMENT_INSET.mul_add(-2.0, outer),
        horizontal,
        text,
    }
}
/// Legacy pointer-sized row, unchanged for every existing caller.
#[must_use]
pub fn segmented(size: SegmentSize) -> Div {
    segmented_at(size, ControlSizing::Pointer)
}
/// Source row with explicit input layout: MD32 pointer/44 touch; SM28 both.
#[must_use]
pub fn segmented_at(size: SegmentSize, sizing: ControlSizing) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(spacing::S1))
        .min_h(px(geometry(size, sizing).outer))
}
/// Legacy pointer segment and paint, unchanged. Callers attach a handler only
/// when enabled and recheck their current selection before changing it.
#[must_use]
pub fn segment(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    chosen: bool,
    enabled: bool,
    size: SegmentSize,
    tokens: &Tokens,
) -> Stateful<Div> {
    render(
        id,
        label,
        chosen,
        enabled,
        Profile {
            size,
            sizing: ControlSizing::Pointer,
            presentation: Presentation::Legacy,
        },
        tokens,
    )
}
/// Source MD28/40 segment inside the32/44 row, or SM24 inside28.
/// Unselected hover uses surface2 and press surface3. Selected stays surface3.
/// As with the legacy entry, callers attach no disabled action and refuse a
/// no-op choice of their currently selected value.
#[must_use]
pub fn segment_at(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    chosen: bool,
    enabled: bool,
    size: SegmentSize,
    sizing: ControlSizing,
    tokens: &Tokens,
) -> Stateful<Div> {
    render(
        id,
        label,
        chosen,
        enabled,
        Profile {
            size,
            sizing,
            presentation: Presentation::Source,
        },
        tokens,
    )
}
fn render(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    chosen: bool,
    enabled: bool,
    profile: Profile,
    tokens: &Tokens,
) -> Stateful<Div> {
    let geometry = geometry(profile.size, profile.sizing);
    let neutral = &tokens.neutral;
    let hover = neutral.surface2;
    let pressed = neutral.surface3;
    div()
        .id(id)
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(spacing::S1))
        .min_h(px(geometry.inner))
        .px(px(geometry.horizontal))
        .rounded_full()
        .type_(Type::Body)
        .text_size(px(crate::fonts::ui(geometry.text)))
        .text_color(if chosen {
            neutral.foreground
        } else {
            neutral.foreground_muted
        })
        .when(chosen, |segment| segment.bg(neutral.surface3))
        .when(!enabled, |segment| segment.opacity(0.5))
        .when(enabled && !chosen, |segment| {
            segment
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .when(
                    matches!(profile.presentation, Presentation::Source),
                    |segment| segment.active(move |style| style.bg(pressed)),
                )
        })
        .child(label.into())
}

#[cfg(test)]
mod tests;
