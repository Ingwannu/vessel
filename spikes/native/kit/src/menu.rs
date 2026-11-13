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
//! Extension: leading glyphs and typed pending labels share the same row;
//! pending items cannot be chosen by pointer or keyboard, even if an item
//! became pending after it was highlighted. Legacy calls stay content-sized.

mod draw;
mod item;
#[cfg(test)]
mod tests;
mod trigger;

pub use crate::select::Anchor as TriggerAnchor;
pub use draw::{Content, Width, menu, menu_with};
use gpui::{App, Bounds, Context, FocusHandle, Pixels, Point, Window};
pub use item::{ItemStatus, MenuItem, Tone};
use std::rc::Rc;
pub use trigger::{TriggerSizing, trigger};

/// A menu's state, kept by the screen that shows it.
#[derive(Debug)]
pub struct MenuState {
    at: Option<Placement>,
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
        self.at = Some(Placement::Pointer(at));
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
    fn step(&mut self, items: &[MenuItem], direction: Direction) {
        self.highlighted = next_index(items, self.highlighted, direction);
    }

    /// Open below the trigger's right edge, as Paseo's align="end".
    pub fn open_end(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
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
    /// A row was chosen (the menu has closed).
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
    fn geometry(self) -> (Point<Pixels>, gpui::Anchor) {
        match self {
            Self::Pointer(at) => (at, gpui::Anchor::TopLeft),
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
    let mut at = from;
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
