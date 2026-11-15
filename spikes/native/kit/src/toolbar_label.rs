//! Paseo's current/base branch trigger (`components/ui/toolbar-label-trigger.tsx`).
//!
//! [Decision Log]
//! - Purpose: a compact toolbar label is not the composer's round picker pill.
//! - Constraints: the reference uses a 12-point muted label, a 12-point caret,
//!   spacing[1] insets/gap, radius md, and interactionHighlight when highlighted.
//! - Alternatives: changing `pill` would alter every existing composer picker;
//!   drawing the trigger in an area would create a second shared control there.
//! - Choice: a separate frame with typed interaction and label layout. The
//!   reference's one-line layout is the default choice; wrapping is explicit
//!   for the Changes head's long branch name, as requested in its review.
//! - Cost: touch uses the kit's minimum target; disabled callers attach no click.

use gpui::prelude::*;
use gpui::{Div, ElementId, SharedString, Stateful, div, px};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::{glyph, one_line};
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;

/// Whether the trigger can open its picker and whether it is already open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Ready to open.
    Ready,
    /// The associated picker is open.
    Open,
    /// Passive context in the same frame, as Paseo's disabled Pressable.
    Disabled,
}

/// How a constrained toolbar gives room to its label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelLayout {
    /// Paseo's `numberOfLines={1}`; truncate only at the reference's last resort.
    SingleLine,
    /// A long current branch can wrap instead of disappearing in the panel head.
    Wrap,
}

/// The reference's borderRadius.md, independent of popup row geometry.
const RADIUS: f32 = 6.0;
/// `ToolbarLabelTriggerIcon`'s `ChevronDown` size.
const CARET: f32 = 12.0;

/// A branch label and its caret. The caller owns its picker, anchor and click;
/// disabled callers keep the same frame and must not attach a click handler.
#[must_use]
pub fn trigger(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    state: State,
    layout: LabelLayout,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let label = one_line(&label.into());
    let highlighted = state == State::Open;
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(spacing::S1))
        .min_w_0()
        .flex_shrink(1.0)
        .px(px(spacing::S1))
        .py(px(spacing::S1))
        .rounded(px(RADIUS))
        .type_(Type::Meta)
        .text_color(if highlighted {
            neutral.foreground
        } else {
            neutral.foreground_muted
        })
        .when(highlighted, |frame| frame.bg(neutral.interaction_highlight))
        .when(touch, |frame| {
            frame.min_h(px(crate::breakpoint::TOUCH_TARGET))
        })
        .when(state != State::Disabled, |frame| {
            frame
                .cursor_pointer()
                .hover(move |style| {
                    style
                        .bg(neutral.interaction_highlight)
                        .text_color(neutral.foreground)
                })
                .active(move |style| {
                    style
                        .bg(neutral.interaction_highlight)
                        .text_color(neutral.foreground)
                })
        })
        .child(
            div()
                .min_w_0()
                .flex_shrink(1.0)
                .when(layout == LabelLayout::SingleLine, Styled::truncate)
                .when(layout == LabelLayout::Wrap, Styled::whitespace_normal)
                .child(label),
        )
        .child(glyph(
            Glyph::ChevronDown,
            CARET,
            neutral.foreground_extra_muted,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trigger_uses_paseos_label_geometry() {
        assert_eq!(spacing::S1.to_bits(), 4.0_f32.to_bits());
        assert_eq!(RADIUS.to_bits(), 6.0_f32.to_bits());
        assert_eq!(CARET.to_bits(), 12.0_f32.to_bits());
        assert_eq!(
            Type::Meta.metrics().0.to_bits(),
            crate::fonts::ui(12.0).to_bits()
        );
    }
}
