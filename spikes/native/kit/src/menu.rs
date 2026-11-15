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
//! placement, keyboard handling or motion. The prior split changed no behaviour;
//! typed feedback and dismissal are additive extensions of that renderer.

mod draw;
#[cfg(test)]
mod feedback_tests;
#[cfg(test)]
mod focus_tests;
mod item;
#[cfg(test)]
mod presentation_tests;
#[cfg(test)]
mod tests;
mod trigger;
pub use crate::select::Anchor as TriggerAnchor;
pub use draw::{Content, Width, menu, menu_right, menu_with};
pub use item::{Dismissal, ItemStatus, LeadingSize, MenuItem, Presentation, Tone};
pub use trigger::{TriggerProfile, TriggerSizing, trigger, trigger_with};

use gpui::{App, Bounds, Context, FocusHandle, Pixels, Point, WeakFocusHandle, Window};
use std::rc::Rc;

/// A menu's state, kept by the screen that shows it.
#[derive(Debug)]
pub struct MenuState {
    at: Option<Placement>,
    highlighted: Option<usize>,
    focus: FocusHandle,
    return_to: Option<WeakFocusHandle>,
}

impl MenuState {
    /// A closed menu.
    pub fn new(cx: &mut App) -> Self {
        Self {
            at: None,
            highlighted: None,
            focus: cx.focus_handle(),
            return_to: None,
        }
    }

    /// Open it at `at` (where its trigger was pressed), taking the keys.
    pub fn open(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
        self.capture_return_focus(window, cx);
        self.at = Some(Placement::Pointer(at));
        self.highlighted = None;
        self.focus.focus(window, cx);
    }

    /// Close state only, preserving the existing API for callers without a
    /// window. Input handlers use `close_and_restore` before dispatching actions.
    pub const fn close(&mut self) {
        self.at = None;
        self.highlighted = None;
    }

    /// Dismiss and restore the caller's focus only while this menu still owns
    /// it. A new field/dialog that took focus is never displaced. A weak handle
    /// cannot keep a removed caller alive.
    pub fn close_and_restore(&mut self, window: &mut Window, cx: &mut App) {
        self.close();
        self.restore_focus(window, cx);
    }
    fn capture_return_focus(&mut self, window: &Window, cx: &App) {
        if !self.is_open() && !self.focus.is_focused(window) {
            self.return_to = window.focused(cx).map(|focus| focus.downgrade());
        }
    }
    fn restore_focus(&mut self, window: &mut Window, cx: &mut App) {
        let previous = self.return_to.take().and_then(|weak| weak.upgrade());
        if !self.focus.is_focused(window) {
            return;
        }
        match previous {
            Some(focus) => focus.focus(window, cx),
            None => window.blur(cx),
        }
    }
    fn restore_after_choice(&mut self, window: &mut Window, cx: &mut App) {
        if !self.is_open() {
            self.restore_focus(window, cx);
        }
    }

    /// Whether it is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.at.is_some()
    }

    /// Move the highlight to the next row that can be chosen, forward or
    /// back, wrapping.
    fn step(&mut self, items: &[MenuItem], direction: Direction) {
        self.highlighted = next_index(items, self.highlighted, direction);
    }

    /// Open at a measured trigger's right/bottom corner, taking the keys.
    pub fn open_end(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        self.capture_return_focus(window, cx);
        self.at = Some(Placement::End(bounds));
        self.highlighted = None;
        self.focus.focus(window, cx);
    }
}

/// What a screen does with a chosen row: the screen, the row's index.
pub type Chose<V> = Rc<dyn Fn(&mut V, usize, &mut Window, &mut Context<'_, V>)>;

/// What a screen hands the menu: how to reach its state, and what to do
/// with a chosen row.
pub struct MenuHandlers<V> {
    /// The screen's menu state.
    pub state: fn(&mut V) -> &mut MenuState,
    /// A row was chosen. Close items dismiss first; `KeepOpen` items retain
    /// this surface so the caller can publish pending/result feedback.
    pub chose: Chose<V>,
}

impl<V> std::fmt::Debug for MenuHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuHandlers").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy)]
enum Placement {
    Pointer(Point<Pixels>),
    End(Bounds<Pixels>),
}
impl Placement {
    fn geometry(self, pointer_corner: gpui::Anchor) -> (Point<Pixels>, gpui::Anchor) {
        match self {
            Self::Pointer(at) => (at, pointer_corner),
            Self::End(bounds) => (
                gpui::point(bounds.right(), bounds.bottom()),
                gpui::Anchor::TopRight,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Next,
    Previous,
}

fn next_index(items: &[MenuItem], from: Option<usize>, direction: Direction) -> Option<usize> {
    let count = items.len();
    if count == 0 {
        return None;
    }
    // An open menu may lose rows after a node update. A stale position must
    // restart at the first/last selectable item, not step through old indices.
    let mut at = from.filter(|index| *index < count);
    for _ in 0..count {
        let next = match (at, direction) {
            (None, Direction::Next) => 0,
            (None | Some(0), Direction::Previous) => count.saturating_sub(1),
            (Some(index), Direction::Next) => {
                index.saturating_add(1).checked_rem(count).unwrap_or(0)
            }
            (Some(index), Direction::Previous) => index.saturating_sub(1),
        };
        at = Some(next);
        if items.get(next).is_some_and(MenuItem::selectable) {
            return at;
        }
    }
    None
}
fn selected_index(items: &[MenuItem], highlighted: Option<usize>) -> Option<usize> {
    highlighted.filter(|index| items.get(*index).is_some_and(MenuItem::selectable))
}

// Pointer and Enter converge here so a KeepOpen action cannot accidentally
// close before its caller publishes pending state. Escape/outside still close.
fn pointer_choice(state: &mut MenuState, item: &MenuItem, index: usize) -> Option<usize> {
    if !item.selectable() {
        return None;
    }
    if item.dismissal == Dismissal::Close {
        state.close();
    }
    Some(index)
}
fn keyboard_choice(state: &mut MenuState, items: &[MenuItem]) -> Option<usize> {
    let index = selected_index(items, state.highlighted)?;
    pointer_choice(state, items.get(index)?, index)
}
