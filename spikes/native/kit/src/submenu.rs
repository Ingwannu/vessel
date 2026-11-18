//! Submenus (TRACKER 791; Paseo's `components/ui/menu/`: `menu-navigation.ts`,
//! `menu-surface.tsx`, `menu-sub.tsx`, and `docs/menus.md`): a menu row that
//! leads to a decision opens its page beside the menu on a wide window, the
//! menu staying open, and in place with a back header in a compact sheet.
//!
//! [Decision Log]
//! - Purpose: one model for the action menu ([`crate::menu`]) and the list
//!   ([`crate::picker`]) alike, so the Display menu and a row menu's Labels
//!   entry open their pages as Paseo's do.
//! - Constraint: a page drawn inside the menu's panel is clipped by it, so a
//!   flyout is its own deferred surface; GPUI runs an element's outside-press
//!   listener in the capture phase, so a flyout cannot stop its parent's
//!   dismissal by stopping propagation.
//! - Choice: [`MenuPath`] is Paseo's navigation (pages from the root outward;
//!   the flyout chain and the sheet's stack are the same path). [`Submenus`],
//!   kept by a screen beside its menu, holds the path, the measured trigger
//!   rows and flyouts, and hover intent: a page opens after the pointer rests
//!   90 ms on its row, closes 260 ms after the pointer leaves, and a pointer
//!   inside the flyout cancels the pending close; a press-only page (a form,
//!   Paseo's `hoverIntent: false`) opens on a press alone and, while open,
//!   stops hover from closing anything. [`flyout`] places a page at its
//!   trigger row's right edge, top on the row, overlapping the menu by 5 so
//!   no strip between them belongs to neither, kept inside the window with
//!   an 8 margin and never flipped (Paseo keeps a submenu's side so it does
//!   not move out from under the pointer travelling to it). A menu with
//!   submenus asks [`Submenus::contains`] before treating a press as outside.
//! - Why: Paseo's one model drives both presentations; a flyout per screen
//!   would be the copies `docs/menus.md` warns drift apart.
//! - Cost: the screen keeps a [`Submenus`] and draws each open page's
//!   flyout after its menu (same layer, so later paints on top). A timer is
//!   owned by the state: replacing or dropping it cancels the old one.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, App, Bounds, ElementId, Pixels, Point, SharedString, Task, Window,
    anchored, deferred, div, point, px,
};

/// How long the pointer rests on a row before its page opens (Paseo's
/// `HOVER_OPEN_DELAY_MS`).
pub const HOVER_OPEN: Duration = Duration::from_millis(90);
/// How long after the pointer leaves before a page closes (Paseo's
/// `HOVER_CLOSE_GRACE_MS`): the pointer crosses sibling rows on its way in.
pub const HOVER_CLOSE: Duration = Duration::from_millis(260);
/// How far a flyout sits over its menu (Paseo's `SUBMENU_OVERLAP`).
pub const OVERLAP: f32 = 5.0;
/// A flyout's margin from the window's edges (Paseo's `EDGE_PADDING`).
pub const EDGE: f32 = 8.0;

/// A page's name, unique within its menu.
pub type PageId = SharedString;

/// The open pages, from the root outward; empty is the root page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuPath(Vec<PageId>);

impl MenuPath {
    /// Open `page` from a trigger `depth` pages deep (0 on the root page):
    /// the path is cut back to that depth first, so moving between two
    /// triggers on one page swaps their pages instead of nesting them.
    pub fn open(&mut self, page: &PageId, depth: usize) {
        if self.is_open(page, depth) {
            return;
        }
        self.0.truncate(depth);
        self.0.push(page.clone());
    }

    /// Close the page opened at `depth` and every page it opened.
    pub fn close(&mut self, depth: usize) {
        self.0.truncate(depth);
    }

    /// Back one page; false at the root (the caller closes the menu).
    pub fn back(&mut self) -> bool {
        self.0.pop().is_some()
    }

    /// Cut the path back to the deepest page that still exists.
    pub fn prune(&mut self, exists: impl Fn(&PageId) -> bool) {
        if let Some(missing) = self.0.iter().position(|page| !exists(page)) {
            self.0.truncate(missing);
        }
    }

    /// The page a sheet shows; `None` is the root.
    #[must_use]
    pub fn current(&self) -> Option<&PageId> {
        self.0.last()
    }

    /// Whether `page`, opened from `depth`, is open.
    #[must_use]
    pub fn is_open(&self, page: &PageId, depth: usize) -> bool {
        self.0.get(depth) == Some(page)
    }

    /// The open pages, from the root outward.
    #[must_use]
    pub fn pages(&self) -> &[PageId] {
        &self.0
    }
}

/// A pending open or close, cancelled by dropping it.
#[derive(Debug)]
struct Timer {
    token: Rc<()>,
    _task: Task<()>,
}

type Probe = Rc<Cell<Option<Bounds<Pixels>>>>;

#[derive(Debug, Default)]
struct Inner {
    path: MenuPath,
    /// Each trigger row, measured where it was drawn.
    triggers: HashMap<PageId, Probe>,
    /// Each open flyout's surface, by the depth of its page.
    surfaces: HashMap<usize, Probe>,
    /// Pages a press opens and hover does not (forms).
    press_only: Vec<PageId>,
    /// A page a press or Enter opened, until its screen takes the keys.
    pressed: Option<PageId>,
    /// The open page the person works in (its screen moved the keys into
    /// it, or a press landed inside it): hover neither closes it nor swaps
    /// it for a sibling (keys would be left on a list no longer drawn); its
    /// own Escape, a press on another row or the menu's close ends it.
    keyed: Option<PageId>,
    opening: Option<Timer>,
    /// The page `opening` opens.
    opening_page: Option<PageId>,
    closing: Option<Timer>,
    /// The menu opening this path belongs to.
    cycle: Option<Weak<()>>,
}

impl Inner {
    /// A press-only page or one holding the keys is open (`keyed` is
    /// settled: it names an open page or none).
    fn held(&self) -> bool {
        self.keyed.is_some()
            || self
                .path
                .pages()
                .iter()
                .any(|page| self.press_only.contains(page))
    }

    /// Drop the keys' hold once its page is no longer open.
    fn settle_keys(&mut self) {
        let closed = self
            .keyed
            .as_ref()
            .is_some_and(|page| !self.path.pages().contains(page));
        if closed {
            self.keyed = None;
        }
    }
}

/// A menu's submenu state, kept by the screen beside its menu; clones share
/// it (the rows' and timers' handles).
#[derive(Debug, Clone, Default)]
pub struct Submenus(Rc<RefCell<Inner>>);

impl Submenus {
    /// No page open.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The open pages.
    #[must_use]
    pub fn path(&self) -> MenuPath {
        self.0.borrow().path.clone()
    }

    /// Whether `page`, opened from `depth`, is open (its row is active).
    #[must_use]
    pub fn is_open(&self, page: &str, depth: usize) -> bool {
        self.0
            .borrow()
            .path
            .is_open(&PageId::from(page.to_owned()), depth)
    }

    /// Mark `page` as one a press opens and hover does not (a form).
    pub fn press_only(&self, page: &str) {
        let mut inner = self.0.borrow_mut();
        let page = PageId::from(page.to_owned());
        if !inner.press_only.contains(&page) {
            inner.press_only.push(page);
        }
    }

    /// A press on `page`'s row (or Enter): open it at once.
    pub fn open(&self, page: &str, depth: usize) {
        let mut inner = self.0.borrow_mut();
        inner.opening = None;
        inner.closing = None;
        let page = PageId::from(page.to_owned());
        inner.path.open(&page, depth);
        inner.settle_keys();
        inner.pressed = Some(page);
    }

    /// Whether `page` was opened by a press or Enter since last asked: its
    /// screen moves the keys into it (a page the pointer opened leaves them
    /// with the menu, as Paseo's flyouts do).
    #[must_use = "true once: the screen moves the keys into the page"]
    pub fn take_pressed(&self, page: &str) -> bool {
        let taken = self.pressed(page);
        if taken {
            let mut inner = self.0.borrow_mut();
            inner.keyed = inner.pressed.take();
        }
        taken
    }

    /// Whether `page` was opened by a press or Enter and its screen has
    /// not taken the keys yet ([`take_pressed`](Self::take_pressed)).
    #[must_use]
    pub fn pressed(&self, page: &str) -> bool {
        let inner = self.0.borrow();
        // A page closed since (Back, the grace) no longer wants the keys.
        inner.pressed.as_deref() == Some(page)
            && inner.path.pages().iter().any(|open| **open == *page)
    }

    /// Back one page; false at the root.
    #[must_use = "false at the root, where the caller closes the menu"]
    pub fn back(&self) -> bool {
        let mut inner = self.0.borrow_mut();
        inner.opening = None;
        inner.closing = None;
        let back = inner.path.back();
        inner.settle_keys();
        back
    }

    /// Close every page (the menu closed).
    pub fn close_all(&self) {
        let mut inner = self.0.borrow_mut();
        inner.opening = None;
        inner.closing = None;
        inner.path = MenuPath::default();
        inner.surfaces.clear();
        inner.pressed = None;
        inner.keyed = None;
    }

    /// Cut the path back to the pages that still exist.
    pub fn prune(&self, exists: impl Fn(&PageId) -> bool) {
        let mut inner = self.0.borrow_mut();
        inner.path.prune(exists);
        inner.settle_keys();
    }

    /// The pointer came to rest on `page`'s row: open it after
    /// [`HOVER_OPEN`], unless it is press-only or a page holds the keys.
    pub fn hover_open(&self, page: &str, depth: usize, window: &Window, cx: &App) {
        let page = PageId::from(page.to_owned());
        let mut inner = self.0.borrow_mut();
        inner.closing = None;
        if inner.press_only.contains(&page) || inner.keyed.is_some() {
            inner.opening = None;
            return;
        }
        inner.opening_page = Some(page.clone());
        let timer = self.after(HOVER_OPEN, window, cx, move |inner| {
            inner.path.open(&page, depth);
            inner.settle_keys();
        });
        inner.opening = Some(timer);
    }

    /// The pointer left a row or flyout at `depth`: close what it opened
    /// after [`HOVER_CLOSE`], unless a press-only page or one holding the
    /// keys is open.
    pub fn hover_close(&self, depth: usize, window: &Window, cx: &App) {
        // Left the row straight into its flyout: in one pointer move the
        // flyout (drawn on top) hears its entering before the row its
        // leaving, so the cancel came first and this close must not start.
        let at = window.mouse_position();
        let into_flyout = self.contains(at);
        let mut inner = self.0.borrow_mut();
        // Left straight onto a sibling page's row, which heard its entering
        // first: its pending opening swaps the branch; cancelling it or
        // closing under it would leave neither page open.
        let onto_sibling = inner
            .opening_page
            .as_ref()
            .and_then(|page| inner.triggers.get(page))
            .and_then(|probe| probe.get())
            .is_some_and(|row| row.contains(&at));
        if onto_sibling {
            return;
        }
        inner.opening = None;
        if inner.held() || into_flyout {
            return;
        }
        let timer = self.after(HOVER_CLOSE, window, cx, move |inner| {
            inner.path.close(depth);
            inner.settle_keys();
        });
        inner.closing = Some(timer);
    }

    /// The pointer is inside a flyout: keep it.
    pub fn cancel_close(&self) {
        self.0.borrow_mut().closing = None;
    }

    /// A press landed inside `page`'s flyout: hold it (see `keyed`).
    fn hold(&self, page: &PageId) {
        let mut inner = self.0.borrow_mut();
        if inner.path.pages().contains(page) {
            inner.closing = None;
            inner.keyed = Some(page.clone());
        }
    }

    /// Whether `at` lies on an open flyout: not an outside press for the
    /// menu under it.
    #[must_use]
    pub fn contains(&self, at: Point<Pixels>) -> bool {
        let inner = self.0.borrow();
        let open = inner.path.pages().len();
        inner
            .surfaces
            .iter()
            .filter(|(depth, _)| **depth < open)
            .any(|(_, probe)| probe.get().is_some_and(|bounds| bounds.contains(&at)))
    }

    /// Records `page`'s trigger row where it is drawn: an absolute layer
    /// over the row.
    #[must_use]
    pub fn trigger_probe(&self, page: &str) -> AnyElement {
        let probe = Rc::clone(
            self.0
                .borrow_mut()
                .triggers
                .entry(PageId::from(page.to_owned()))
                .or_default(),
        );
        crate::picker::anchor_probe(&probe)
    }

    /// Where `page`'s trigger row was last drawn.
    #[must_use]
    pub fn trigger(&self, page: &str) -> Option<Bounds<Pixels>> {
        self.0
            .borrow()
            .triggers
            .get(&PageId::from(page.to_owned()))
            .and_then(|probe| probe.get())
    }

    /// Bind the path to a menu opening (`cycle`): a new opening starts at
    /// the root with nothing pending.
    pub(crate) fn follow(&self, cycle: &Weak<()>) {
        let same = self
            .0
            .borrow()
            .cycle
            .as_ref()
            .is_some_and(|held| Weak::ptr_eq(held, cycle));
        if !same {
            self.close_all();
            self.0.borrow_mut().cycle = Some(Weak::clone(cycle));
        }
    }

    fn surface_probe(&self, depth: usize) -> Probe {
        Rc::clone(self.0.borrow_mut().surfaces.entry(depth).or_default())
    }

    /// Run `change` after `delay`, redrawing the window, unless the timer
    /// was replaced or dropped first.
    fn after(
        &self,
        delay: Duration,
        window: &Window,
        cx: &App,
        change: impl FnOnce(&mut Inner) + 'static,
    ) -> Timer {
        let weak = Rc::downgrade(&self.0);
        let token = Rc::new(());
        let delivered = Rc::downgrade(&token);
        let handle = window.window_handle();
        let task = cx.spawn(async move |cx| {
            cx.background_executor().timer(delay).await;
            let _closed = handle.update(cx, |_, window, _cx| {
                let Some(inner) = weak.upgrade() else {
                    return;
                };
                let mut inner = inner.borrow_mut();
                let current = [&inner.opening, &inner.closing]
                    .into_iter()
                    .flatten()
                    .any(|timer| Weak::ptr_eq(&Rc::downgrade(&timer.token), &delivered));
                // The fired timer stays in its slot (dropping a task from
                // inside its own run is avoided); the next intent replaces it.
                if current {
                    change(&mut inner);
                    window.refresh();
                }
            });
        });
        Timer { token, _task: task }
    }
}

/// `page`'s surface (opened from `depth`) beside its trigger row: right of
/// it, top on it, over the menu by [`OVERLAP`], kept inside the window,
/// painted at `priority` (its menu's layer; draw it after the menu). `None`
/// until the row has been drawn once.
#[must_use]
pub fn flyout(
    submenus: &Submenus,
    page: &str,
    depth: usize,
    priority: usize,
    content: impl IntoElement,
) -> Option<AnyElement> {
    let row = submenus.trigger(page)?;
    let at = point(px(OVERLAP.mul_add(-1.0, f32::from(row.right()))), row.top());
    let probe = submenus.surface_probe(depth);
    let entered = submenus.clone();
    let pressed = submenus.clone();
    let held = PageId::from(page.to_owned());
    let surface = div()
        .id(ElementId::Name(SharedString::from(format!(
            "submenu-{page}"
        ))))
        .relative()
        .occlude()
        .on_hover(move |inside: &bool, window, cx| {
            if *inside {
                entered.cancel_close();
            } else {
                entered.hover_close(depth, window, cx);
            }
        })
        .capture_any_mouse_down(move |_event, _window, _cx| pressed.hold(&held))
        .child(content)
        .child(crate::picker::anchor_probe(&probe));
    let placed = anchored()
        .anchor(Anchor::TopLeft)
        .position(at)
        .snap_to_window_with_margin(px(EDGE))
        .child(surface);
    Some(deferred(placed).with_priority(priority).into_any_element())
}

#[cfg(test)]
#[path = "submenu/path_tests.rs"]
mod path_tests;
#[cfg(test)]
#[path = "submenu/tests.rs"]
mod tests;
