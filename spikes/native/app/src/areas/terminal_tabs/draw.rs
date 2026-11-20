//! A terminal tab's frame: rebuilt only when its bytes, viewport, colours,
//! font, cell or cursor phase changed (the session screen's rule, ADR
//! 0073); its size fitted to the pane and claimed for the shell while the
//! pane has the keyboard (Paseo's `terminal-pane-focus-claim.ts`: a focused
//! pane claims its size; resizes settle for 100 ms first). From protocol 37
//! (ADR 0116) the node keeps one owner per terminal: the pane claims when
//! it takes the keyboard or attaches, and its later fits are updates the
//! node drops once another client claimed the shell since.

use std::rc::Rc;
use std::time::Duration;

use vessel_app_kit::Scheme;
use vessel_app_kit::terminal_grid::{CellSize, GridCursor, GridFill, GridFrame, GridRow};
use vessel_app_kit::ui::{App, Bounds, Context, Pixels, point, px, size};
use vessel_app_view::terminal::grid::{Generation, Grid, Scroll, TextAt};
use vessel_app_view::terminal::{FoundMatch, GridSize, span_on};
use vessel_app_view::terminal::paint::rows;
use vessel_proto::ResizeIntent;

use super::{Blink, PaneKey, Stage};
use crate::areas::terminal::convert;
use crate::areas::terminal::font_size;
use crate::areas::terminal::replay::refit;
use crate::shell::Shell;

/// Paseo's `TERMINAL_RESIZE_DEBOUNCE_MS`.
const SETTLE: Duration = Duration::from_millis(100);
/// The cursor's blink half-period (xterm's 600 ms is the session screen's
/// 500, kept the same in the app).
const BLINK: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq)]
/// Every input of a drawn frame.
pub(crate) struct DrawKey {
    generation: Generation,
    blink: Blink,
    scheme: Scheme,
    font: Pixels,
    cell: CellSize,
    scroll: Scroll,
    /// The selection's and find's marks (`Pane::marks`).
    marks: u64,
}

impl Shell {
    /// Rebuild `key`'s frame when something it draws changed.
    pub(super) fn terminal_tab_draw(&mut self, key: &PaneKey, cx: &mut Context<'_, Self>) {
        let tokens = vessel_app_kit::tokens(cx);
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        let Some(model) = pane.model.as_ref() else {
            return;
        };
        pane.viewport.refresh(model.grid());
        let scroll = pane.viewport.offset(model.grid());
        let now = DrawKey {
            generation: model.grid().generation(),
            blink: pane.blink,
            scheme: tokens.scheme,
            font: font_size(),
            cell: pane.grid.read(cx).cell,
            scroll,
            marks: pane.marks,
        };
        if pane.drawn == Some(now) {
            return;
        }
        pane.drawn = Some(now);
        let palette = convert::palette(&tokens.terminal);
        let snapshot = model.snapshot(scroll);
        let cursor = snapshot
            .cursor
            .filter(|_| pane.blink == Blink::On && pane.stage == Stage::Live)
            .map(|cell| GridCursor {
                row: cell.row,
                col: cell.col,
                color: convert::hsla(palette.cursor),
            });
        let mut grid_rows: Vec<GridRow> =
            rows(&snapshot, &palette).iter().map(convert::row).collect();
        let grid = model.grid();
        if let Some(kept) = super::select::live_selection(pane.selected, grid) {
            let color = gpui::rgba(tokens.terminal.selection).into();
            let range = kept.selection.range(grid);
            mark_rows(&mut grid_rows, grid, scroll, |row_start, cols| {
                span_on(range, row_start, cols)
                    .map(|span| (span, color))
                    .into_iter()
                    .collect()
            }, Layer::Fill);
        }
        if let Some(find) = pane.find.as_mut() {
            let (matches, current) = find.marks();
            let other = convert::hsla(palette.ansi[3]);
            let active = convert::hsla(palette.ansi[4]);
            let shown = visible(matches, grid, scroll);
            mark_rows(&mut grid_rows, grid, scroll, |row_start, cols| {
                shown
                    .iter()
                    .filter_map(|(index, found)| {
                        let color = if Some(*index) == current { active } else { other };
                        span_on((found.start, found.end), row_start, cols)
                            .map(|span| (span, color))
                    })
                    .collect()
            }, Layer::Outline);
            // Paseo's `findWidgetCoversSelection`: the bar goes to the
            // bottom when the current match is under it.
            let cell = pane.grid.read(cx).cell;
            let (rows_covered, cols_covered) = super::find_bar::covered(cell);
            let cols = grid.size().cols();
            find.bottom = current.and_then(|at| matches.get(at)).is_some_and(|found| {
                grid.view_row(found.start, scroll)
                    .is_some_and(|row| row < rows_covered)
                    && found.end.col() >= cols.saturating_sub(cols_covered)
            });
        }
        let frame = GridFrame {
            rows: grid_rows,
            cursor,
            background: convert::hsla(palette.background),
            font_size: font_size(),
        };
        pane.grid.update(cx, |grid, cx| {
            grid.frame = Rc::new(frame);
            cx.notify();
        });
    }

    /// The pane's box was measured: fit the grid to it, then (after the
    /// box held for `SETTLE`) claim the size for the shell.
    pub(super) fn terminal_tab_measured(
        &mut self,
        key: &PaneKey,
        cell: CellSize,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if pane.grid.read(cx).cell != cell {
            pane.grid.update(cx, |grid, cx| {
                grid.cell = cell;
                cx.notify();
            });
        }
        let fitted = pane.probe.get().and_then(|bounds| {
            let (cols, rows) = cell.fit(bounds.size);
            GridSize::within(cols, rows).ok()
        });
        if let Some(fit) = fitted
            && pane.fit != Some(fit)
        {
            pane.fit = Some(fit);
            if let Some(model) = pane.model.as_mut() {
                refit(model, Some(fit));
            }
            let settled = key.clone();
            pane.settling = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(SETTLE).await;
                drop(this.update(cx, |shell, _cx| shell.terminal_tab_claim(&settled)));
            }));
        }
        self.terminal_tab_draw(key, cx);
        // A refit re-wraps the text: find's matches are searched again.
        self.terminal_find_output(key, cx);
    }

    /// Send the pane's size while it has the keyboard: a claim first after
    /// it took the keyboard or attached (even at the size last sent, since
    /// another client may have taken the shell meanwhile), then an update
    /// whenever the fit differs from what was sent (Paseo's focus claim and
    /// `terminal-size-ownership.ts`).
    pub(super) fn terminal_tab_claim(&mut self, key: &PaneKey) {
        if self.areas.terminal_tabs.focused.as_ref() != Some(key) {
            return;
        }
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        let Some(fit) = pane.fit else {
            return;
        };
        if pane.stage != Stage::Live {
            return;
        }
        let intent = if !pane.claimed {
            ResizeIntent::Claim
        } else if pane.sent_size != Some(fit) {
            ResizeIntent::Update
        } else {
            return;
        };
        pane.claimed = true;
        pane.sent_size = Some(fit);
        pane.sender.resize(fit.cols(), fit.rows(), intent);
        self.terminal_tab_pump(key);
    }

    /// The focused pane's cursor blinks; any other's is steady.
    pub(super) fn terminal_tab_blink(&mut self, cx: &mut Context<'_, Self>) {
        let focused = self.areas.terminal_tabs.focused.clone();
        let running = self
            .areas
            .terminal_tabs
            .blinking
            .as_ref()
            .map(|(key, _)| key.clone());
        if running == focused && !cx.reduce_motion() {
            return;
        }
        if let Some(previous) = running {
            self.areas.terminal_tabs.blinking = None;
            if let Some(pane) = self.areas.terminal_tabs.panes.get_mut(&previous) {
                pane.blink = Blink::On;
            }
            self.terminal_tab_draw(&previous, cx);
        }
        let Some(key) = focused.filter(|_| !cx.reduce_motion()) else {
            return;
        };
        let blinking = key.clone();
        let task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                let going = this.update(cx, |shell, cx| {
                    if shell.areas.terminal_tabs.focused.as_ref() != Some(&blinking)
                        || cx.reduce_motion()
                    {
                        return false;
                    }
                    let Some(pane) = shell.areas.terminal_tabs.panes.get_mut(&blinking) else {
                        return false;
                    };
                    pane.blink = match pane.blink {
                        Blink::On => Blink::Off,
                        Blink::Off => Blink::On,
                    };
                    shell.terminal_tab_draw(&blinking, cx);
                    true
                });
                if !matches!(going, Ok(true)) {
                    break;
                }
            }
        });
        self.areas.terminal_tabs.blinking = Some((key, task));
    }

    /// Where the cursor's cell is drawn, inside `element` (the pane's box):
    /// the IME's candidate window opens there.
    pub(crate) fn terminal_tab_cursor(
        &self,
        key: &PaneKey,
        element: Bounds<Pixels>,
        cx: &App,
    ) -> Option<Bounds<Pixels>> {
        let pane = self.areas.terminal_tabs.panes.get(key)?;
        let cursor = pane.model.as_ref()?.grid().cursor();
        let cell = pane.grid.read(cx).cell;
        let along = |origin: Pixels, step: Pixels, cells: u16| {
            px(f32::from(step).mul_add(f32::from(cells), f32::from(origin)))
        };
        Some(Bounds {
            origin: point(
                along(element.origin.x, cell.width, cursor.col),
                along(element.origin.y, cell.height, cursor.row),
            ),
            size: size(cell.width, cell.height),
        })
    }
}

/// Where marks go on a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    /// Under the text (a selection's translucent fill).
    Fill,
    /// Around the cells, over the text (find's borders).
    Outline,
}

/// Add each view row's marks, as `spans` names them for the row starting
/// at a cell, to `rows`; a span ending on a wide scalar covers both its
/// cells.
fn mark_rows(
    rows: &mut [GridRow],
    grid: &Grid,
    scroll: Scroll,
    spans: impl Fn(TextAt, u16) -> Vec<((u16, u16), gpui::Hsla)>,
    layer: Layer,
) {
    let cols = grid.size().cols();
    for (view_row, row) in (0_u16..).zip(rows.iter_mut()) {
        let row_start = grid.text_at(scroll, view_row, 0);
        for ((first, last), color) in spans(row_start, cols) {
            let last = wide_end(grid, row_start.at_col(last), cols);
            let mark = GridFill {
                col: first,
                cells: last.saturating_sub(first).saturating_add(1),
                color,
            };
            match layer {
                Layer::Fill => row.fills.push(mark),
                Layer::Outline => row.outlines.push(mark),
            }
        }
    }
}

/// `at`'s column, or the next when `at` is the left half of a wide scalar.
fn wide_end(grid: &Grid, at: TextAt, cols: u16) -> u16 {
    let next = at.col().saturating_add(1);
    if next < cols && grid.lead_of(at.at_col(next)) == at {
        next
    } else {
        at.col()
    }
}

/// The matches that can show in the view, with their indexes: from the
/// first that ends at or after the view's top to the last that starts
/// before its bottom.
fn visible<'a>(
    matches: &'a [FoundMatch],
    grid: &Grid,
    scroll: Scroll,
) -> Vec<(usize, &'a FoundMatch)> {
    let rows = grid.size().rows();
    let top = grid.text_at(scroll, 0, 0);
    let bottom = grid.text_at(scroll, rows.saturating_sub(1), grid.size().cols().saturating_sub(1));
    let from = matches.partition_point(|found| found.end < top);
    matches
        .iter()
        .enumerate()
        .skip(from)
        .take_while(|(_, found)| found.start <= bottom)
        .collect()
}
