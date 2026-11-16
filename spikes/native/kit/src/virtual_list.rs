//! A long list of rows of any height that draws only what is near the
//! view: a session's timeline (20,000 entries), a diff, a file tree.
//!
//! [Decision Log]
//! - Purpose: one list for every long thing, so following the end,
//!   keeping a row in view and going to a row behave alike everywhere
//!   (the page's `features/session/use-window.ts` and `data/window.ts`).
//! - Constraint: rows are not uniform (a message of three paragraphs, a
//!   tool card, a one-line meta row) and change height as they grow; a
//!   screen must not know GPUI's list to draw one.
//! - Alternatives: the page's window arithmetic on our own scroll view
//!   (`view::timeline::window`): a layout pass per frame over spacers,
//!   which GPUI's list already does with a sum tree of measured heights;
//!   `uniform_list` (one height for every row: wrong for a conversation);
//!   `gpui-component`'s list (a delegate made for pickers, with search
//!   and selection built in).
//! - Choice: GPUI's `list` under an interface of ours: the screen says
//!   how many rows there are, which changed, and draws a row by index;
//!   the list measures what it draws and guesses the rest. Following
//!   the end is the list's tail mode: on while the view is at the end,
//!   off when the person scrolls up, on again when they come back down.
//! - Differences from the page, kept: the page follows while within 48
//!   pixels of the end; GPUI's tail mode takes the end exactly. Going to
//!   a row puts it under the view's top with the page's 16-pixel inset;
//!   "centre" (find's match) is the list's own "reveal" (the row brought
//!   fully into view), as GPUI does not say where an unmeasured row
//!   would centre.
//! - Cost: the rows are drawn through a closure the list keeps, so a
//!   row's drawing must read the screen's state by index, not capture it.

use std::ops::Range;

use gpui::{
    AnyElement, App, Bounds, FollowMode, List, ListAlignment, ListOffset, ListState, Pixels,
    Styled as _, Window, list, px,
};

/// How far past the view rows are drawn and measured, each way (the
/// page's window draws about 1,200 pixels around the view).
const OVERDRAW: f32 = 1200.0;

/// The gap left above a row put at the view's top (the page's `INSET`),
/// as the offset into the row the view starts at: above it.
const INSET_ABOVE: f32 = -16.0;

/// The page's `INSET`: the gap kept between something brought into view
/// and the view's edge.
const INSET: f32 = 16.0;

/// Where a row goes when the list goes to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    /// Its top just under the view's top (the outline's jump).
    Top,
    /// Brought fully into view, moving as little as it can (find's
    /// match).
    Nearest,
}

/// A long list's state, kept by the screen that shows it. Cheap to clone:
/// the clones share one list.
#[derive(Clone)]
pub struct VirtualList {
    state: ListState,
}

impl std::fmt::Debug for VirtualList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualList")
            .field("rows", &self.state.item_count())
            .finish_non_exhaustive()
    }
}

impl VirtualList {
    /// An empty list. `follow_end`: a log that grows at its end and
    /// follows it while the view is there (a conversation, a terminal's
    /// scrollback); otherwise a list read from its top (a tree, a diff).
    #[must_use]
    pub fn new(follow_end: bool) -> Self {
        let state = ListState::new(0, ListAlignment::Top, px(OVERDRAW));
        if follow_end {
            state.set_follow_mode(FollowMode::Tail);
        }
        Self { state }
    }

    /// The list now has `rows` rows, none of them measured (another
    /// session opened, the rows folded differently). A following list
    /// goes to its end.
    pub fn reset(&self, rows: usize) {
        self.state.reset(rows);
        if self.state.is_following_tail() {
            self.state.scroll_to_end();
        }
    }

    /// Rows `old` were replaced by `count` rows (appended rows: an empty
    /// range at the end).
    pub fn splice(&self, old: Range<usize>, count: usize) {
        self.state.splice(old, count);
    }

    /// Rows `range` changed and are to be measured again where they
    /// stand (a message grew, a tool call finished); the view keeps its
    /// place.
    pub fn changed(&self, range: Range<usize>) {
        if !range.is_empty() {
            self.state.remeasure_items(range);
        }
    }

    /// Observe handled scroll input on the existing list. Replaces its prior
    /// handler; install once for the owning view instance. This notification
    /// does not promise that a clamped list moved.
    ///
    /// GPUI borrows the list state during this callback. Do not query or mutate
    /// this `VirtualList` inside it; defer such work to the owner instead.
    /// No row geometry or feature capability is inferred from the notification.
    pub fn on_scroll(&self, mut handler: impl FnMut(&mut Window, &mut App) + 'static) {
        self.state.set_scroll_handler(move |_event, window, cx| {
            handler(window, cx);
        });
    }

    /// How many rows the list has.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.state.item_count()
    }

    /// Go to row `row`.
    pub fn reveal(&self, row: usize, how: Reveal) {
        match how {
            Reveal::Top => self.state.scroll_to(ListOffset {
                item_ix: row,
                offset_in_item: px(INSET_ABOVE),
            }),
            Reveal::Nearest => self.state.scroll_to_reveal_item(row),
        }
    }

    /// Bring `bounds` into view, moving as little as it can, with the
    /// page's 16-pixel inset, and `above` clear at the top for what floats
    /// over the list there (the find bar): a line inside a row taller
    /// than the view, in window coordinates (a reply's renderer reports
    /// find's current match so, `TextView::on_reveal`).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "pixel geometry: positions on screen, far from any overflow"
    )]
    pub fn reveal_bounds(&self, bounds: Bounds<Pixels>, above: Pixels) {
        let view = self.state.viewport_bounds();
        let inset = px(INSET);
        let top = view.top() + inset.max(above);
        let bottom = view.bottom() - inset;
        if bounds.top() < top {
            self.state.scroll_by(bounds.top() - top);
        } else if bounds.bottom() > bottom {
            self.state.scroll_by(bounds.bottom() - bottom);
        }
    }

    /// Scroll by `distance` (negative: up, which stops following the
    /// end).
    pub fn scroll_by(&self, distance: Pixels) {
        self.state.scroll_by(distance);
    }

    /// Go to the end and follow it.
    pub fn to_end(&self) {
        self.state.scroll_to_end();
    }

    /// Whether the view is following the end.
    #[must_use]
    pub fn following(&self) -> bool {
        self.state.is_following_tail()
    }

    /// The view's width, once laid out (zero before).
    #[must_use]
    pub fn width(&self) -> Pixels {
        self.state.viewport_bounds().size.width
    }

    /// The first row in view, and how far into it the view starts.
    #[must_use]
    pub fn top(&self) -> (usize, Pixels) {
        let offset = self.state.logical_scroll_top();
        (offset.item_ix, offset.offset_in_item)
    }

    /// The list, drawing row `index` with `row`. It fills its parent.
    pub fn element(
        &self,
        row: impl FnMut(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> List {
        list(self.state.clone(), row).size_full()
    }
}

#[cfg(test)]
#[path = "virtual_list_scroll_tests.rs"]
mod scroll_tests;
