//! A menu row's tooltip (TRACKER 803): Paseo's `MenuItem` `tooltip`
//! (`components/ui/menu/menu-item.tsx`), first for the tab menu's Reload
//! agent ("Reload agent to update skills, MCPs or login status.").
//!
//! [Decision Log]
//! - Purpose and intent: a row says more than its label while the pointer
//!   rests on it, as Paseo's row does.
//! - Existing implementation and constraints: the kit tooltip
//!   (`crate::tooltip::Tooltips`) is kept by a screen and drawn by it over
//!   everything; a menu is drawn in its own deferred layer by whichever
//!   screen opens it, and its rows are made fresh each frame.
//! - Alternatives considered: the screen's tooltips threaded into
//!   `Content` (every menu caller would hold and draw one); GPUI's own
//!   `tooltip` (it waits and sits by the pointer, not beside the row).
//! - Chosen approach: the menu's state keeps its own `Tooltips` and a place
//!   per row; a row with words attaches them with Paseo's placement
//!   (`Configuration::menu_row`: right of the row, centred, 10 away, after
//!   250 ms), and the menu draws the tip inside its own layer, after its
//!   surface. A phone draws none (Paseo's `enabledOnMobile={false}`).
//! - Pros, cons and impact: callers only set `MenuItem::tooltip`; a tip
//!   lives only while its menu is drawn, and an opening takes down the
//!   last one. Rows of a page drawn by the screen (a flyout's list) do not
//!   take part.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, Stateful, div};

use super::{MenuItem, MenuState};
use crate::theme::Tokens;
use crate::tooltip::{Configuration, Place};

impl MenuState {
    /// Where row `index` was last drawn, kept across frames.
    fn tip_place(&self, index: usize) -> Place {
        let mut places = self.tip_places.borrow_mut();
        if places.len() <= index {
            places.resize_with(index.saturating_add(1), Place::default);
        }
        places.get(index).map(Rc::clone).unwrap_or_default()
    }
}

/// `row` with `item`'s tooltip, when it has one and the menu is not a
/// phone's.
pub(super) fn with_tip(
    row: Stateful<gpui::Div>,
    index: usize,
    item: &MenuItem,
    state: &MenuState,
    narrow: bool,
) -> Stateful<gpui::Div> {
    match &item.tooltip {
        Some(words) if !narrow => state.tips.on_configured(
            row,
            &state.tip_place(index),
            words.clone(),
            Configuration::menu_row(),
            crate::ControlSizing::Pointer,
        ),
        Some(_) | None => row,
    }
}

/// `surface` with the tooltip shown drawn after it, in the same layer.
/// The holder is out of the flow, as the anchored surface is, so it moves
/// nothing beside the menu.
pub(super) fn with_layer(surface: AnyElement, state: &MenuState, tokens: &Tokens) -> AnyElement {
    match state.tips.render_in_layer(tokens) {
        Some(tip) => div()
            .absolute()
            .top_0()
            .left_0()
            .child(surface)
            .child(tip)
            .into_any_element(),
        None => surface,
    }
}
