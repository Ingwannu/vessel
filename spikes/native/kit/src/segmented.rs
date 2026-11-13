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
//! few, all in view). Until the kit's theme carries Paseo's surfaces, the
//! chosen fill is `bg_active` and the hovered `bg_hover`, the nearest of
//! the theme's.
//!
//! The control draws; the caller owns the value and is told of a choice
//! through each segment's click, as the kit's other controls do.

use gpui::prelude::*;
use gpui::{Div, ElementId, SharedString, Stateful, div, px};

use crate::fonts::Type;
use crate::theme::Tokens;
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

/// The row that holds the segments.
#[must_use]
pub fn segmented(size: SegmentSize) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(4.0))
        .min_h(px(size.height()))
}

/// One segment; `chosen` fills it, `enabled` false dims it (the caller adds
/// `on_click` only to enabled ones).
#[must_use]
pub fn segment(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    chosen: bool,
    enabled: bool,
    size: SegmentSize,
    tokens: &Tokens,
) -> Stateful<Div> {
    let (padding, text) = match size {
        SegmentSize::Medium => (12.0, 14.0),
        SegmentSize::Small => (8.0, 12.0),
    };
    let hover = tokens.bg_hover;
    div()
        .id(id)
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .min_h(px(size.inner()))
        .px(px(padding))
        .rounded_full()
        .type_(Type::Body)
        .text_size(px(text))
        .text_color(if chosen {
            tokens.text_primary
        } else {
            tokens.text_secondary
        })
        .when(chosen, |segment| segment.bg(tokens.bg_active))
        .when(!enabled, |segment| segment.opacity(0.5))
        .when(enabled && !chosen, |segment| {
            segment.cursor_pointer().hover(move |style| style.bg(hover))
        })
        .child(label.into())
}
