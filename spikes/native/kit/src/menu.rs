//! A menu of actions behind a trigger (DESIGN.md section 6.13; the page's
//! `ui/kit/menu.tsx`): the Session actions, a machine's, a project's.
//!
//! [Decision Log]
//! - Purpose: one menu for every screen, with Paseo's look (the owner's
//!   rule: Paseo's app, cloned; its `components/ui/menu/`): rows of 14
//!   points, 28 tall (44 on a phone), inset 4 from the edges at radius 6;
//!   a hovered or keyboard-highlighted row on `surface2`, never a ring; a
//!   full-width rule between groups; the destructive row in
//!   `destructive`; arrow keys (wrapping), Home and End, Enter or Space,
//!   Escape and a click outside to close.
//! - Alternatives: `gpui-component`'s popup menu (its own theme, actions
//!   and key contexts registered app-wide, submenus we do not need yet;
//!   screens would then depend on its types).
//! - Choice: the menu's state (open where, which row is highlighted, its
//!   focus) is a value the screen keeps; the menu is drawn from it each
//!   frame, over everything, at the point it was opened from, kept
//!   inside the window. A chosen row is handed back by index, so the
//!   screen keeps its own list of what each row does.
//! - Cost: no submenus, group labels or check rows yet (no menu of ours
//!   needs them); a phone shows the same popover with 44-point rows, as
//!   Paseo's menus do unless they ask for a sheet (none of ours does yet).
//!
//! Integration decision: the published renderer owns both entry points.
//! Keep item values, caller-owned state and drawing in separate modules so
//! additive feedback can stay within the file budget without duplicating
//! placement, keyboard handling or motion. This split changes no behaviour.

// The sized-button entry and the future trigger share the same density type.
pub use crate::control_sizing::ControlSizing as TriggerSizing;

mod draw;
mod item;
pub use draw::{menu, menu_right};
pub use item::{MenuItem, Tone};

use gpui::{App, Context, FocusHandle, Pixels, Point, Window};
use std::rc::Rc;

/// A menu's state, kept by the screen that shows it.
#[derive(Debug)]
pub struct MenuState {
    at: Option<Point<Pixels>>,
    highlighted: Option<usize>,
    focus: FocusHandle,
}

impl MenuState {
    /// A closed menu.
    pub fn new(cx: &mut App) -> Self {
        Self {
            at: None,
            highlighted: None,
            focus: cx.focus_handle(),
        }
    }

    /// Open it at `at` (where its trigger was pressed), taking the keys.
    pub fn open(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
        self.at = Some(at);
        self.highlighted = None;
        self.focus.focus(window, cx);
    }

    /// Close it.
    pub const fn close(&mut self) {
        self.at = None;
        self.highlighted = None;
    }

    /// Whether it is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.at.is_some()
    }

    /// Move the highlight to the next row that can be chosen, forward or
    /// back, wrapping.
    fn step(&mut self, items: &[MenuItem], forward: bool) {
        let count = items.len();
        if count == 0 {
            return;
        }
        let mut at = self.highlighted;
        for _ in 0..count {
            let next = match (at, forward) {
                (None, true) => 0,
                (None | Some(0), false) => count.saturating_sub(1),
                (Some(index), true) => index.saturating_add(1).checked_rem(count).unwrap_or(0),
                (Some(index), false) => index.saturating_sub(1),
            };
            at = Some(next);
            if items.get(next).is_some_and(|item| !item.disabled) {
                self.highlighted = at;
                return;
            }
        }
    }
}

/// What a screen does with a chosen row: the screen, the row's index.
pub type Chose<V> = Rc<dyn Fn(&mut V, usize, &mut Window, &mut Context<'_, V>)>;

/// What a screen hands the menu: how to reach its state, and what to do
/// with a chosen row.
pub struct MenuHandlers<V> {
    /// The screen's menu state.
    pub state: fn(&mut V) -> &mut MenuState,
    /// A row was chosen (the menu has closed).
    pub chose: Chose<V>,
}

impl<V> std::fmt::Debug for MenuHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuHandlers").finish_non_exhaustive()
    }
}
