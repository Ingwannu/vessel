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
mod input;
mod item;
#[cfg(test)]
mod lease_tests;
#[cfg(test)]
mod modal_tests;
#[cfg(test)]
mod presentation_tests;
#[cfg(test)]
mod shortcut_tests;
mod sub_row;
#[cfg(test)]
mod submenu_tests;
#[cfg(test)]
mod tests;
mod trigger;
pub use crate::select::Anchor as TriggerAnchor;
pub use draw::{Content, Layer, Width, menu, menu_right, menu_with};
pub use item::{Dismissal, ItemStatus, LeadingSize, MenuItem, Presentation, Tone};
pub use trigger::{
    TriggerPresentation, TriggerProfile, TriggerSizing, trigger, trigger_presented, trigger_space,
    trigger_visual_size, trigger_with,
};

use gpui::{App, Bounds, Context, FocusHandle, Pixels, Point, Window};
use std::rc::{Rc, Weak};

/// A menu's state, kept by the screen that shows it.
#[derive(Debug)]
pub struct MenuState {
    at: Option<Placement>,
    highlighted: Option<usize>,
    focus: FocusHandle,
    return_to: crate::focus::ReturnFocus,
    cycle: Rc<()>,
}

impl MenuState {
    /// A closed menu.
    pub fn new(cx: &mut App) -> Self {
        Self {
            at: None,
            highlighted: None,
            focus: cx.focus_handle(),
            return_to: crate::focus::ReturnFocus::default(),
            cycle: Rc::new(()),
        }
    }

    /// Open it at `at` (where its trigger was pressed), taking the keys.
    pub fn open(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
        self.capture_return_focus(window, cx);
        self.cycle = Rc::new(());
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
        if !self.is_open() {
            // State-only legacy close cannot restore without a Window, but a
            // later opening is a new cycle and must not retain its old caller.
            self.return_to = crate::focus::ReturnFocus::default();
            self.return_to.remember(window, cx);
        }
    }
    fn restore_focus(&mut self, window: &mut Window, cx: &mut App) {
        self.return_to.restore_if_owned(&self.focus, window, cx);
    }
    fn restore_after_choice(&mut self, window: &mut Window, cx: &mut App) {
        if !self.is_open() {
            self.restore_focus(window, cx);
        }
    }

    // Each explicit opening has allocation identity, rather than a wrapping
    // counter. Retained Weak leases keep that allocation's header alive, so a
    // replacement state/opening cannot reuse it while an old handler survives.
    fn render_lease(&self) -> RenderLease {
        RenderLease(Rc::downgrade(&self.cycle))
    }
    fn accepts(&self, lease: &RenderLease) -> bool {
        self.is_open() && self.same_cycle(lease)
    }
    fn same_cycle(&self, lease: &RenderLease) -> bool {
        Weak::ptr_eq(&lease.0, &Rc::downgrade(&self.cycle))
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
        self.cycle = Rc::new(());
        self.at = Some(Placement::End(bounds));
        self.highlighted = None;
        self.focus.focus(window, cx);
    }

    /// Open under a measured trigger with its left edges aligned (Paseo's
    /// `side="bottom" align="start"`, the new-tab menu), taking the keys.
    pub fn open_start(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        self.capture_return_focus(window, cx);
        self.cycle = Rc::new(());
        self.at = Some(Placement::Start(bounds));
        self.highlighted = None;
        self.focus.focus(window, cx);
    }
}

// Entry requires an open cycle. After an authorized Close choice, identity
// alone permits that choice's focus restoration/action; it does not authorize
// another callback to enter the now-closed cycle.
#[derive(Debug, Clone)]
struct RenderLease(Weak<()>);

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
    Start(Bounds<Pixels>),
}
impl Placement {
    fn geometry(self, pointer_corner: gpui::Anchor) -> (Point<Pixels>, gpui::Anchor) {
        match self {
            Self::Pointer(at) => (at, pointer_corner),
            Self::End(bounds) => (
                gpui::point(bounds.right(), bounds.bottom()),
                gpui::Anchor::TopRight,
            ),
            Self::Start(bounds) => (
                gpui::point(bounds.left(), bounds.bottom()),
                gpui::Anchor::TopLeft,
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
    // A row that opens a page navigates (the renderer opens it); it neither
    // closes the menu nor reaches the caller.
    if !item.selectable() || item.opens.is_some() {
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
