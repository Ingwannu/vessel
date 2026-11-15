//! The select: a setting's value with a caret, opening the list of its
//! values under it (Paseo's `SettingsSelect`: `components/ui/
//! dropdown-trigger.tsx` over its dropdown menu, `side="bottom"`,
//! `align="end"`, 220 wide, a check on the value in effect).
//!
//! [Decision Log]
//! - Purpose: a setting with a few named values (the send shortcut, the
//!   default send, a size, the conversation's width) is chosen from a
//!   list, never from words with a "▾" (the owner's rule after 0.0.0.0.3).
//! - Reference: Paseo's trigger is the outline button's chrome: a
//!   hairline of `borderAccent` on clear, fully round, 32 tall (44 on
//!   touch), 16 across (24 on touch), the value in 14 `foreground` and a 14
//!   muted caret 8 after it, 85% while pressed and half-faded when
//!   disabled; its list opens under it, its right edge on the trigger's.
//! - Choice: the list is the kit's picker list (`picker.rs`'s
//!   [`crate::combobox`]: checked rows, a sheet on a phone, the room
//!   under the trigger as its height), not a second list; this file draws
//!   the trigger and says where the list opens. A screen keeps one
//!   [`crate::ComboboxState`] for all its selects and which one is open,
//!   as the composer's controls share one.
//! - Alternatives: track C's menu (`menu.rs`): Paseo's dropdown, but it has
//!   no checked row, and a select without its check reads as a menu of
//!   actions.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{Bounds, Div, ElementId, Pixels, SharedString, Stateful, div, point, px};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::{ComboHeader, Side, anchor_probe, glyph};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// Paseo's select list is 220 wide.
pub const WIDTH: f32 = 220.0;

/// Where a trigger was last drawn, in the window.
pub type Anchor = Rc<Cell<Option<Bounds<Pixels>>>>;

/// The trigger: `label` (the value in effect) and a caret; where it is
/// drawn goes to `anchor`. `open`: its list is showing. `touch`: sized for
/// a finger. The screen handles its click (open the list).
#[must_use]
pub fn trigger(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    anchor: &Anchor,
    open: bool,
    enabled: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    div()
        .id(id)
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .flex_shrink_0()
        .max_w_full()
        .gap(px(8.0))
        .min_h(px(if touch { 44.0 } else { 32.0 }))
        .px(px(if touch { 24.0 } else { 16.0 }))
        .rounded_full()
        .border_1()
        .border_color(neutral.border_accent)
        .type_(Type::Body)
        .text_color(neutral.foreground)
        .when(open, |trigger| trigger.opacity(0.85))
        .when(enabled, |trigger| {
            trigger.cursor_pointer().active(|style| style.opacity(0.85))
        })
        .when(!enabled, |trigger| trigger.opacity(0.5))
        .child(anchor_probe(anchor))
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(label.into()),
        )
        .child(glyph(Glyph::ChevronDown, 14.0, neutral.foreground_muted))
}

/// The bounds to open a list `width` wide from, for a trigger drawn at
/// `bounds`: Paseo's `align="end"`, the list's right edge on the
/// trigger's (the picker list opens at its anchor's left edge; it keeps
/// itself inside the window).
#[must_use]
pub fn end_aligned(bounds: Bounds<Pixels>, width: f32) -> Bounds<Pixels> {
    let left = width.mul_add(-1.0, f32::from(bounds.right())).max(0.0);
    Bounds {
        origin: point(px(left), bounds.origin.y),
        size: bounds.size,
    }
}

/// A select's list header: no title, no search, under its trigger.
#[must_use]
pub fn header() -> ComboHeader {
    ComboHeader {
        title: None,
        back: false,
        search: false,
        empty: SharedString::default(),
        note: None,
        side: Side::Below,
    }
}
