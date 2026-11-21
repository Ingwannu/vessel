//! Selecting and copying in a terminal tab: xterm.js's desktop mouse
//! selection, which Paseo's desktop terminal keeps, and Paseo's copy keys
//! (`terminal-emulator-runtime.ts` `handleCtrlClipboardShortcut`).
//!
//! [Decision Log]
//! - Purpose: a person selects output with the mouse and copies it, as in
//!   Paseo's desktop terminal (P3).
//! - Choice: the view's `Selection` (anchored to its text, so output that
//!   scrolls keeps it on its lines); a press starts it (one click cells,
//!   two words, three a line; Shift extends the one there), a drag moves
//!   its free end, past the top or bottom edge a line of scrolling a move.
//!   Copy writes its text (soft wraps joined, trailing blanks dropped) to
//!   the clipboard; Paseo shows no toast and neither does this.
//! - Clears it: typing or a paste (xterm.js's `onData`), a plain click, a
//!   width change (the text re-wraps under it) and the other screen.
//! - Not taken: Paseo's mobile grid (long press, grips, floating Copy), the
//!   phone app's later (coordinator, P3); Paseo desktop's context menu is
//!   its Electron shell's, whose items are not in its source.

use gpui::{ClipboardItem, MouseDownEvent, Pixels, Point};
use vessel_app_view::terminal::grid::{Scroll, Shown, TextAt};
use vessel_app_view::terminal::{SelectBy, Selection, TerminalModel};

use super::PaneKey;
use crate::shell::Shell;

/// A selection and what it was made on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Selected {
    pub(crate) selection: Selection,
    shown: Shown,
    cols: u16,
}

impl Selected {
    /// `selection` as made on what `grid` shows now.
    pub(crate) const fn on(
        selection: Selection,
        grid: &vessel_app_view::terminal::grid::Grid,
    ) -> Self {
        Self {
            selection,
            shown: grid.shown(),
            cols: grid.size().cols(),
        }
    }
}

/// Where the pointer is against the screen's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Above,
    Inside,
    Below,
}

impl Shell {
    /// A press on a tab's screen: a new selection, or Shift extending the
    /// one there. The press also gives the tab the keyboard (the caller).
    pub(super) fn terminal_tab_press(
        &mut self,
        key: &PaneKey,
        event: &MouseDownEvent,
        cx: &mut gpui::Context<'_, Self>,
    ) {
        let Some((at, _)) = self.terminal_tab_cell(key, event.position, cx) else {
            return;
        };
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        let Some(grid) = pane.model.as_ref().map(TerminalModel::grid) else {
            return;
        };
        let (shown, cols) = (grid.shown(), grid.size().cols());
        let extend = event.modifiers.shift
            && pane
                .selected
                .is_some_and(|kept| kept.shown == shown && kept.cols == cols);
        if extend && let Some(kept) = pane.selected.as_mut() {
            kept.selection.extend(at);
        } else {
            let by = SelectBy::of_clicks(event.click_count);
            pane.selected = Some(Selected::on(Selection::new(at, by), grid));
        }
        pane.selecting = true;
        pane.marked();
        self.terminal_tab_draw(key, cx);
    }

    /// The pointer moved while a press selects. With the button still
    /// down the selection follows it; with it up, the release was missed
    /// and the press ends here (TRACKER 836: a release outside the screen
    /// before the frame that listens window-wide left the drag open, and
    /// the next press elsewhere extended it).
    pub(super) fn terminal_tab_pointer_moved(
        &mut self,
        key: &PaneKey,
        event: &gpui::MouseMoveEvent,
        cx: &mut gpui::Context<'_, Self>,
    ) {
        if event.pressed_button == Some(gpui::MouseButton::Left) {
            self.terminal_tab_drag(key, event.position, cx);
        } else {
            self.terminal_tab_release(key, cx);
        }
    }

    /// The pointer moved while a press selects: the free end follows it,
    /// and past the top or bottom edge the view scrolls a line toward it.
    pub(super) fn terminal_tab_drag(
        &mut self,
        key: &PaneKey,
        position: Point<Pixels>,
        cx: &mut gpui::Context<'_, Self>,
    ) {
        if !self
            .areas
            .terminal_tabs
            .panes
            .get(key)
            .is_some_and(|pane| pane.selecting)
        {
            return;
        }
        let Some((at, edge)) = self.terminal_tab_cell(key, position, cx) else {
            return;
        };
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if let Some(grid) = pane.model.as_ref().map(TerminalModel::grid) {
            match edge {
                Edge::Above => pane.viewport.earlier(Scroll(1), grid),
                Edge::Below => pane.viewport.later(Scroll(1), grid),
                Edge::Inside => {}
            }
        }
        if let Some(kept) = pane.selected.as_mut()
            && kept.selection.head() != at
        {
            kept.selection.extend(at);
            pane.marked();
        }
        self.terminal_tab_draw(key, cx);
    }

    /// The press ended: a press of cells that was not dragged selects
    /// nothing (a plain click clears).
    pub(super) fn terminal_tab_release(&mut self, key: &PaneKey, cx: &mut gpui::Context<'_, Self>) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if !std::mem::replace(&mut pane.selecting, false) {
            return;
        }
        if pane.selected.is_some_and(|kept| kept.selection.is_empty()) {
            pane.selected = None;
            pane.marked();
            self.terminal_tab_draw(key, cx);
        }
    }

    /// Copy the selection to the clipboard. Whether there was one to copy
    /// (Ctrl+C without one is the shell's ^C).
    pub(super) fn terminal_tab_copy(
        &mut self,
        key: &PaneKey,
        cx: &mut gpui::Context<'_, Self>,
    ) -> bool {
        let Some(pane) = self.areas.terminal_tabs.panes.get(key) else {
            return false;
        };
        let Some(grid) = pane.model.as_ref().map(TerminalModel::grid) else {
            return false;
        };
        let Some(kept) = live_selection(pane.selected, grid) else {
            return false;
        };
        let text = kept.selection.text(grid);
        if text.is_empty() {
            return false;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    /// Drop the selection (typing, a paste); whether there was one.
    pub(super) fn terminal_tab_unselect(&mut self, key: &PaneKey) -> bool {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return false;
        };
        let had = pane.selected.take().is_some();
        if had {
            pane.marked();
        }
        had
    }

    /// The cell under `position` on `key`'s screen, clamped into it, and
    /// whether the pointer is above or below the screen.
    #[expect(clippy::float_arithmetic, reason = "a pointer's place in cells")]
    fn terminal_tab_cell(
        &self,
        key: &PaneKey,
        position: Point<Pixels>,
        cx: &gpui::App,
    ) -> Option<(TextAt, Edge)> {
        let pane = self.areas.terminal_tabs.panes.get(key)?;
        let bounds = pane.probe.get()?;
        let cell = pane.grid.read(cx).cell;
        let grid = pane.model.as_ref()?.grid();
        let size = grid.size();
        let x = f32::from(position.x) - f32::from(bounds.origin.x);
        let y = f32::from(position.y) - f32::from(bounds.origin.y);
        let edge = if y < 0.0 {
            Edge::Above
        } else if y >= f32::from(cell.height) * f32::from(size.rows()) {
            Edge::Below
        } else {
            Edge::Inside
        };
        let row = whole(y, f32::from(cell.height), size.rows());
        let col = whole(x, f32::from(cell.width), size.cols());
        let scroll = pane.viewport.offset(grid);
        Some((grid.text_at(scroll, row, col), edge))
    }
}

/// The selection, if it still belongs to what `grid` shows.
pub(super) fn live_selection(
    selected: Option<Selected>,
    grid: &vessel_app_view::terminal::grid::Grid,
) -> Option<Selected> {
    selected.filter(|kept| {
        kept.shown == grid.shown() && kept.cols == grid.size().cols() && !kept.selection.is_empty()
    })
}

/// The cell `offset` falls in, of `count` cells `size` long, clamped.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_arithmetic,
    reason = "floored and clamped to 0 ..= count - 1 before the cast"
)]
fn whole(offset: f32, size: f32, count: u16) -> u16 {
    if size <= 0.0 || count == 0 {
        return 0;
    }
    let last = f32::from(count.saturating_sub(1));
    (offset / size).floor().clamp(0.0, last) as u16
}
