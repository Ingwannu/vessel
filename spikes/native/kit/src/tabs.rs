//! A side panel's tabs (the page's `ui/kit/panel-tabs.tsx`, DESIGN.md
//! 6.13): agent-orchestrator's inspector head, 36px over a hairline, its
//! tabs 28px tall, 8px corners, 4px apart, in the passive ink; hovered on
//! a 4% wash, chosen in the full ink with no fill; a count rides on a
//! tab's corner (past 99, "99+").
//!
//! [Decision Log]
//! - Purpose: the inspector beside a session switches what it shows
//!   (Summary, Changes, Files); track C's Summary and this track's code
//!   panel share the strip.
//! - Constraint: the page's tabs are icons only, named by tooltips; the
//!   kit has no icon set yet (its only drawings are the window's four
//!   glyphs), and no tooltip (the lead's).
//! - Choice: a tab carries its label, drawn as the tab until the kit has
//!   icons, then as the icon with the label as its tooltip; the strip and
//!   its measures are the page's either way.
//! - Cost: labelled tabs are wider than the page's squares; three fit in
//!   the narrowest inspector (360 points).

use gpui::prelude::*;
use gpui::{Div, ElementId, SharedString, Stateful, div, px};

use crate::fonts::Type;
use crate::theme::{Tokens, radius};
use crate::widgets::TypeExt as _;

/// The strip's height (`--topbar-h`).
pub const HEAD_HEIGHT: f32 = 36.0;

/// The head the tabs sit in, with the panel's own actions at its right
/// (`end`).
#[must_use]
pub fn tab_head(tabs: Div, end: Option<Div>, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(4.0))
        .h(px(HEAD_HEIGHT))
        .pl(px(4.0))
        .pr(px(6.0))
        .border_b_1()
        .border_color(tokens.border_faint)
        .child(
            tabs.flex()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .min_w(px(0.0)),
        )
        .children(end.map(|end| {
            end.flex()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .ml_auto()
                .min_w(px(0.0))
        }))
}

/// A count as the badge writes it.
#[must_use]
pub fn count_text(count: u64) -> String {
    if count > 99 {
        "99+".to_owned()
    } else {
        count.to_string()
    }
}

/// One tab: its label, whether it is chosen, and a count (none at zero).
#[must_use]
pub fn tab(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    chosen: bool,
    count: Option<u64>,
    tokens: &Tokens,
) -> Stateful<Div> {
    let hover = tokens.bg_hover;
    div()
        .id(id)
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(radius::R2))
        .type_(Type::Label)
        .text_size(px(13.0))
        .text_color(if chosen {
            tokens.text_primary
        } else {
            tokens.text_tertiary
        })
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(label.into())
        .children(count.filter(|count| *count > 0).map(|count| {
            div()
                .absolute()
                .top(px(-2.0))
                .right(px(-4.0))
                .min_w(px(12.0))
                .h(px(12.0))
                .px(px(2.0))
                .rounded_full()
                .bg(tokens.text_primary)
                .text_color(tokens.bg_base)
                .type_(Type::Mono)
                .text_size(px(7.0))
                .line_height(px(12.0))
                .flex()
                .justify_center()
                .child(count_text(count))
        }))
}

#[cfg(test)]
mod tests {
    use super::count_text;

    #[test]
    fn writes_a_count_as_the_badge_does() {
        assert_eq!(count_text(7), "7");
        assert_eq!(count_text(99), "99");
        assert_eq!(count_text(100), "99+");
    }
}
