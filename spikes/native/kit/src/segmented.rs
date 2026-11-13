//! A segmented control (the page's `ui/kit/segmented.tsx`, DESIGN.md
//! 6.13): one of two to four choices, all in view. agent-orchestrator's
//! settings segment (a 2px-inset track on the field fill, with its edge;
//! 13px items, 32px tall), the chosen segment GOAT's thumb lifted by a
//! hairline; `Small` is the 24px toolbar size.
//!
//! The control draws; the caller owns the value and is told of a choice
//! through each segment's click, as the kit's other controls do. Keys
//! (arrows moving the choice) come with the kit's focus handling.

use gpui::prelude::*;
use gpui::{Div, ElementId, SharedString, Stateful, div, px};

use crate::fonts::Type;
use crate::theme::{Tokens, radius};
use crate::widgets::TypeExt as _;

/// The control's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentSize {
    /// 32px, a page's or a dialog's.
    Medium,
    /// 24px, a toolbar's.
    Small,
}

/// The track that holds the segments.
#[must_use]
pub fn segmented(size: SegmentSize, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .border_1()
        .border_color(tokens.field_edge)
        .bg(tokens.field_bg)
        .rounded(px(match size {
            SegmentSize::Medium => radius::R5,
            SegmentSize::Small => radius::R2,
        }))
}

/// One segment; `chosen` lifts it, `enabled` false greys it and takes no
/// click (the caller adds `on_click` only to enabled ones).
#[must_use]
pub fn segment(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    chosen: bool,
    enabled: bool,
    size: SegmentSize,
    tokens: &Tokens,
) -> Stateful<Div> {
    let (height, padding, corner, text) = match size {
        SegmentSize::Medium => (32.0, 12.0, radius::R4, 13.0),
        SegmentSize::Small => (24.0, 10.0, 6.0, 12.0),
    };
    let hover = tokens.text_primary;
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(height))
        .px(px(padding))
        .rounded(px(corner))
        .type_(Type::Body)
        .text_size(px(text))
        .line_height(px(text))
        .text_color(if chosen {
            tokens.text_primary
        } else {
            tokens.text_secondary
        })
        .when(chosen, |segment| {
            segment
                .bg(tokens.segment_thumb)
                .border_1()
                .border_color(tokens.segment_edge)
        })
        .when(!enabled, |segment| segment.opacity(0.5))
        .when(enabled, |segment| {
            segment
                .cursor_pointer()
                .hover(move |style| style.text_color(hover))
        })
        .child(label.into())
}
