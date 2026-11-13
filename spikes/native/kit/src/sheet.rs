//! A sheet: a panel over the screen from one edge, with a scrim behind it
//! that closes it (ADR 0036: below the phone's width the sidebar is a
//! sheet from the title bar; below the inspector's width the inspector is
//! one from the right; a phone's choices come up from the bottom).
//!
//! [Decision Log]
//! - Purpose: one experience from 360 points wide to a desktop; what sits
//!   beside the page on a wide window comes over it on a narrow one.
//! - Constraint: GPUI has no sheet; `gpui-component` has a drawer, but
//!   screens are written against Vessel's kit (ADR 0035), and its drawer
//!   brings its own look and focus rules.
//! - Alternatives: the component layer's drawer behind the kit (its look
//!   and its root's overlay layer, which the kit would have to follow);
//!   GPUI's `deferred`/`anchored` overlay (positions against a point, not
//!   an edge).
//! - Choice: an absolute layer over its parent (which must be
//!   `relative()`): a scrim that closes on a press, and the panel at the
//!   edge, which occludes the scrim so a press in it does not close it.
//!   No animation yet.
//! - Must stay true: nothing in a sheet depends on hover; its rows are the
//!   same as beside the page.

pub mod snap;

use gpui::prelude::*;
use gpui::{AnyElement, App, Div, MouseButton, MouseDownEvent, Window, div, px, rgba};

use crate::theme::Tokens;

/// Which edge a sheet comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// The left: the sidebar.
    Left,
    /// The right: an inspector.
    Right,
    /// The bottom: a phone's choices.
    Bottom,
}

/// The widest a side sheet is, in points (the sidebar's width beside the
/// page); on a narrower window it leaves a strip of the scrim to press.
const SIDE: f32 = 300.0;

/// The scrim's colour: the page behind, dimmed.
const SCRIM: u32 = 0x0000_0040;

/// `content` in a sheet from `edge` over the parent; `on_dismiss` runs on
/// a press on the scrim.
#[must_use]
pub fn sheet(
    id: &'static str,
    edge: Edge,
    content: AnyElement,
    tokens: &Tokens,
    on_dismiss: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let panel = div()
        .id(id)
        .absolute()
        .flex()
        .flex_col()
        .bg(tokens.bg_shell)
        .border_color(tokens.border_subtle)
        .occlude()
        .child(content);
    let panel = match edge {
        Edge::Left => panel
            .top_0()
            .bottom_0()
            .left_0()
            .w(px(SIDE))
            .max_w(gpui::relative(0.85))
            .border_r_1(),
        Edge::Right => panel
            .top_0()
            .bottom_0()
            .right_0()
            .w(px(SIDE))
            .max_w(gpui::relative(0.85))
            .border_l_1(),
        Edge::Bottom => panel
            .left_0()
            .right_0()
            .bottom_0()
            .max_h(gpui::relative(0.85))
            .border_t_1()
            .rounded_t(px(crate::theme::radius::R5)),
    };
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(rgba(SCRIM))
                .on_mouse_down(MouseButton::Left, on_dismiss),
        )
        .child(panel)
}
