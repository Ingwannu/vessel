//! Dragging a project's header to put the sidebar's projects in the
//! person's own order (TRACKER 811), as Paseo's (`sidebar-workspace-list.tsx`,
//! the projects' `DraggableList`, @dnd-kit 6.3.1's sortable list).
//!
//! [Decision Log]
//! - Purpose: Paseo's projects stand where the person dragged them; the
//!   native tree listed them by name only.
//! - How it goes (Paseo's `useDragReorderState` and dnd-kit's defaults):
//!   - A press on a header that moves 6 starts a GPUI drag
//!     (`LIST_DRAG_DISTANCE`, Paseo's `distance: 6`); one that does not is
//!     a press (the fold). GPUI forgets the press's click once a drag
//!     starts, so the release never folds.
//!   - At the start the order shown, each block's centre and the dragged
//!     block's height are kept, the centres in the list's own coordinates
//!     (as if nothing were scrolled): lists arriving mid-drag do not
//!     reshuffle it, and scrolling does not move the targets (dnd-kit
//!     measures its droppables when the drag starts).
//!   - As the pointer moves, the kept centre nearest the lifted block's
//!     centre is the one it is over (`closestCenter`), and the order is the
//!     kept one with the dragged project moved there (`arrayMove`). The
//!     list is drawn in that order with an empty gap of the block's height
//!     where it would land, and the block itself (header and open rows,
//!     drawn by the same code) over the list at the pointer's height less
//!     where it was grabbed, at the list's left: vertical only, at 0.9, its
//!     header in `surface2` with a 1 `border` edge and the theme's medium
//!     shadow (Paseo's `projectRowDragging`).
//!   - A release anywhere writes the order when it changed
//!     (`ProjectOrder::dropped`, the coordinator's Q2). Escape cancels: the
//!     order shown at the start comes back and GPUI's drag ends. Escape is
//!     heard through an app-wide interceptor, as the hover card's, since the
//!     keys go to whatever has focus.
//!   - Held in the outer 20% of the list's height, the list scrolls, faster
//!     the deeper the pointer is (dnd-kit's pointer auto-scroll: 10 points
//!     per 5 ms at the zone's inner edge times the depth), in ticks of
//!     16 ms.
//!   - The hover card closes as the drag starts. GPUI sends no hover while
//!     a drag is under way, so no other opens.
//! - Alternatives: GPUI's drag view as the lifted block (an entity of its
//!   own: it cannot draw the block from the shell's state, and it follows
//!   the pointer on both axes); the block hopping from slot to slot (ruled
//!   out, Q1).
//! - Differences, named: no scale (GPUI has no transform for a laid-out
//!   block); the other blocks move to the gap at once (no 200 ms slide);
//!   pointer only, with no announcements.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::Task;
use vessel_app_kit::ScrollState;
use vessel_app_kit::drag::LIST_DRAG_DISTANCE;
use vessel_app_kit::ui::{
    AppContext as _, Bounds, Context, Div, InteractiveElement as _, IntoElement, MouseButton,
    Pixels, Render, Stateful, StatefulInteractiveElement as _, Subscription, Window, div, px,
};
use vessel_app_link::Command;
use vessel_app_view::prefs::{ProjectKey, ProjectOrder};
use vessel_app_view::sidebar::tree::SidebarProject;

use crate::shell::Shell;

/// The outer share of the list's height where a held drag scrolls it
/// (dnd-kit's `threshold`, 20%).
const EDGE: f32 = 0.2;
/// How far it scrolls in a tick at the zone's inner edge, times the
/// pointer's depth: dnd-kit's 10 points per 5 ms (`acceleration`,
/// `interval`), over a tick of 16 ms.
const SPEED: f32 = 32.0;
const TICK: Duration = Duration::from_millis(16);
/// The lifted block (Paseo's `opacity: 0.9`).
pub(super) const LIFTED: f32 = 0.9;
/// The list's own place, where its top is read.
pub(super) const LIST: &str = "project-list";

/// What a header carries while dragged: its project.
#[derive(Debug, Clone)]
pub(super) struct ProjectDrag(ProjectKey);

/// GPUI's own drag view: nothing (the list draws the lifted block).
struct Unseen;

impl Render for Unseen {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

/// The project tree's state kept by the sidebar between frames.
#[derive(Default)]
pub(in crate::areas::sidebar) struct ProjectsKept {
    /// The projects showing every session (Show more pressed; not kept
    /// across launches, as Paseo's).
    pub(in crate::areas::sidebar) shown_all: super::ShownAll,
    /// The drag under way.
    pub(super) dragging: Option<Dragging>,
    /// Where the last press on a header was (its height in the window):
    /// the block follows the pointer from there, as dnd-kit's does, not
    /// from where the press had moved 6.
    pressed: Rc<Cell<Option<Pixels>>>,
}

/// A drag under way.
pub(super) struct Dragging {
    /// The project dragged.
    pub(super) key: ProjectKey,
    /// The order shown when it started (Paseo's `dragItems`).
    before: Vec<ProjectKey>,
    /// The order it would leave now.
    pub(super) order: ProjectOrder,
    /// Each block's centre when it started, in the list's coordinates, in
    /// `before`'s order.
    centres: Vec<Pixels>,
    /// The dragged block's height (its gap's).
    pub(super) height: Pixels,
    /// How far below the block's top it was grabbed.
    grab: Pixels,
    /// The list's top, in its own coordinates.
    list_top: Pixels,
    /// The pointer's height in the window.
    pointer: Pixels,
    // Held for their lifetime: Escape's interceptor and the edge scroll's
    // ticker end with the drag.
    _escape: Subscription,
    _scroll: Task<()>,
}

/// The place a project's block records its bounds in.
pub(super) fn block_place(key: &ProjectKey) -> String {
    format!("block:{}/{}", key.machine, key.project)
}

/// `header` starts dragging `project` once a press on it moves 6.
pub(super) fn draggable(
    header: Stateful<Div>,
    project: &SidebarProject<'_>,
    shell: &Shell,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let pressing = Rc::clone(&shell.areas.sidebar.projects.pressed);
    let pressed = Rc::clone(&pressing);
    let this = cx.weak_entity();
    let key = ProjectKey::new(project.machine, project.project.id);
    header
        .on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
            pressing.set(Some(event.position.y));
        })
        .drag_threshold(LIST_DRAG_DISTANCE)
        .on_drag(ProjectDrag(key), move |dragged, at, _window, cx| {
            let key = dragged.0.clone();
            let press = pressed.take();
            drop(this.update(cx, |shell, cx| {
                shell.project_drag_started(key, at.y, press, cx);
            }));
            cx.new(|_| Unseen)
        })
}

impl Dragging {
    /// The lifted block's top, from the list's top as drawn now.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "finite on-screen pixel geometry"
    )]
    pub(super) fn lifted_top(&self, scroll: &ScrollState) -> Pixels {
        let list = scroll.bounds().top() + scroll.offset().y + self.list_top;
        self.pointer - self.grab - list
    }

    /// The lifted block's centre in the list's coordinates.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "finite on-screen pixel geometry"
    )]
    fn lifted_centre(&self, scroll: &ScrollState) -> Pixels {
        let centre = self.pointer - self.grab + self.height / 2.0;
        in_list(centre, scroll)
    }
}

impl Shell {
    /// A header's drag started with the pointer `at` below its top, after
    /// a press at `press` (its height in the window).
    fn project_drag_started(
        &mut self,
        key: ProjectKey,
        at: Pixels,
        press: Option<Pixels>,
        cx: &mut Context<'_, Self>,
    ) {
        let before = super::shown_keys(self);
        let sidebar = &self.areas.sidebar;
        let scroll = &sidebar.layout.scroll;
        let Some(block) = sidebar.place(&block_place(&key)).get() else {
            return;
        };
        let centres = before
            .iter()
            .map(|shown| {
                sidebar
                    .place(&block_place(shown))
                    .get()
                    .map_or(Pixels::ZERO, |bounds| in_list(bounds.center().y, scroll))
            })
            .collect();
        let list_top = sidebar
            .place(LIST)
            .get()
            .map_or(Pixels::ZERO, |bounds| in_list(bounds.top(), scroll));
        let pointer = below(block.top(), at);
        let grab = press.map_or(at, |press| from_top(press, block.top()));
        self.areas.sidebar.cards.close();
        let this = cx.weak_entity();
        let escape = cx.intercept_keystrokes(move |event, window, cx| {
            if event.keystroke.key == "escape" {
                drop(this.update(cx, |shell, cx| shell.project_drag_cancelled(window, cx)));
            }
        });
        let ticks = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let going = this.update(cx, |shell, cx| shell.project_drag_scrolled(cx));
                if !matches!(going, Ok(true)) {
                    break;
                }
            }
        });
        self.areas.sidebar.projects.dragging = Some(Dragging {
            order: ProjectOrder::of(before.clone()),
            key,
            before,
            centres,
            height: block.size.height,
            grab,
            list_top,
            pointer,
            _escape: escape,
            _scroll: ticks,
        });
        cx.notify();
    }

    /// The pointer moved to `pointer` (its height) while a project is
    /// dragged.
    pub(super) fn project_drag_moved(&mut self, pointer: Pixels, cx: &mut Context<'_, Self>) {
        let Some(drag) = self.areas.sidebar.projects.dragging.as_mut() else {
            return;
        };
        drag.pointer = pointer;
        self.project_drag_over();
        cx.notify();
    }

    /// The order the drag would leave, from where the lifted block is.
    fn project_drag_over(&mut self) {
        let sidebar = &mut self.areas.sidebar;
        let Some(drag) = sidebar.projects.dragging.as_mut() else {
            return;
        };
        let centre = drag.lifted_centre(&sidebar.layout.scroll);
        let from = drag.before.iter().position(|key| *key == drag.key);
        if let (Some(from), Some(to)) = (from, closest(&drag.centres, centre)) {
            drag.order = ProjectOrder::of(moved(&drag.before, from, to));
        }
    }

    /// The pointer was released: the order the drag leaves is written when
    /// it changed.
    pub(super) fn project_dropped(&mut self, cx: &mut Context<'_, Self>) {
        let Some(drag) = self.areas.sidebar.projects.dragging.take() else {
            return;
        };
        let order = &mut self.prefs.settings.project_order;
        if order.dropped(&drag.before, drag.order.keys()) {
            self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        }
        cx.notify();
    }

    /// Escape during a drag: the order shown at its start comes back.
    fn project_drag_cancelled(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.sidebar.projects.dragging.take().is_some() {
            let _stopped = cx.stop_active_drag(window);
            cx.notify();
        }
    }

    /// A tick of the edge scroll; whether a drag is still under way.
    fn project_drag_scrolled(&mut self, cx: &mut Context<'_, Self>) -> bool {
        let sidebar = &self.areas.sidebar;
        let Some(drag) = sidebar.projects.dragging.as_ref() else {
            return false;
        };
        let scroll = &sidebar.layout.scroll;
        let delta = edge_scroll(scroll.bounds(), drag.pointer);
        if delta != Pixels::ZERO && scroll.scroll_by(delta) {
            self.project_drag_over();
            cx.notify();
        }
        true
    }
}

/// `y` in the window, in the list's coordinates (as if nothing were
/// scrolled).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite on-screen pixel geometry"
)]
fn in_list(y: Pixels, scroll: &ScrollState) -> Pixels {
    y - scroll.bounds().top() - scroll.offset().y
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite on-screen pixel geometry"
)]
fn below(top: Pixels, by: Pixels) -> Pixels {
    top + by
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite on-screen pixel geometry"
)]
fn from_top(y: Pixels, top: Pixels) -> Pixels {
    y - top
}

/// The index of the centre nearest `centre` (dnd-kit's `closestCenter`).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite on-screen pixel geometry"
)]
fn closest(centres: &[Pixels], centre: Pixels) -> Option<usize> {
    centres
        .iter()
        .enumerate()
        .min_by(|(_, one), (_, two)| {
            (**one - centre)
                .abs()
                .partial_cmp(&(**two - centre).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
}

/// `keys` with the one at `from` moved to `to` (dnd-kit's `arrayMove`).
fn moved(keys: &[ProjectKey], from: usize, to: usize) -> Vec<ProjectKey> {
    let mut keys = keys.to_vec();
    if from < keys.len() && to < keys.len() {
        let key = keys.remove(from);
        keys.insert(to, key);
    }
    keys
}

/// How far a pointer at `y` scrolls the list in a tick: toward the start
/// (negative) in the top zone, toward the end in the bottom one, else not.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::float_arithmetic,
    reason = "finite on-screen pixel geometry; the speed scales with the depth"
)]
fn edge_scroll(list: Bounds<Pixels>, y: Pixels) -> Pixels {
    let zone = list.size.height * EDGE;
    if zone <= Pixels::ZERO {
        return Pixels::ZERO;
    }
    let top = list.top() + zone;
    let bottom = list.bottom() - zone;
    if y <= top {
        px(-SPEED * ((top - y) / zone))
    } else if y >= bottom {
        px(SPEED * ((y - bottom) / zone))
    } else {
        Pixels::ZERO
    }
}

#[cfg(test)]
#[path = "drag_tests.rs"]
mod tests;
