//! Appearance and viewport identity invalidate a quiet retained frame too.

use std::rc::Rc;
use vessel_app_kit::ui::{Context, Pixels};
use vessel_app_kit::{
    Scheme,
    terminal_grid::{CellSize, GridCursor, GridFrame},
};
use vessel_app_view::terminal::{
    GridSize,
    grid::{Generation, Scroll},
    paint::rows,
};

use super::{Blink, SETTLE, convert, font_size, replay::refit};
use crate::shell::Shell;

#[derive(Debug, Clone, Copy, PartialEq)]
/// Every retained visual input, including changes without terminal bytes.
pub(super) struct DrawKey {
    generation: Generation,
    blink: Blink,
    scheme: Scheme,
    font: Pixels,
    cell: CellSize,
    scroll: Scroll,
}

impl Shell {
    /// Rebuild only when bytes, viewport, appearance or cursor phase changed.
    pub(super) fn terminal_draw(&mut self, cx: &mut Context<'_, Self>) {
        let tokens = vessel_app_kit::tokens(cx);
        let area = &mut self.areas.terminal;
        let Some(model) = area.model.as_ref() else {
            return;
        };
        area.viewport.refresh(model.grid());
        let scroll = area.viewport.offset(model.grid());
        let now = DrawKey {
            generation: model.grid().generation(),
            blink: area.blink,
            scheme: tokens.scheme,
            font: font_size(),
            cell: area.grid.read(cx).cell,
            scroll,
        };
        if area.drawn == Some(now) {
            return;
        }
        area.drawn = Some(now);
        let palette = convert::palette(&tokens.terminal);
        let snapshot = model.snapshot(scroll);
        let cursor = snapshot
            .cursor
            .filter(|_| area.blink == Blink::On)
            .map(|cell| GridCursor {
                row: cell.row,
                col: cell.col,
                color: convert::hsla(palette.cursor),
            });
        let frame = GridFrame {
            rows: rows(&snapshot, &palette).iter().map(convert::row).collect(),
            cursor,
            background: convert::hsla(palette.background),
            font_size: font_size(),
        };
        area.grid.update(cx, |grid, cx| {
            grid.frame = Rc::new(frame);
            cx.notify();
        });
    }

    /// Appearance is refreshed before the equal-size fast path. A held
    /// cursor and quiet program must never leave yesterday's palette cached.
    pub(super) fn terminal_measured(&mut self, cell: CellSize, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        if area.grid.read(cx).cell != cell {
            area.grid.update(cx, |grid, cx| {
                grid.cell = cell;
                cx.notify();
            });
        }
        self.terminal_draw(cx);
        let area = &mut self.areas.terminal;
        let Some(bounds) = area.probe.get() else {
            return;
        };
        let (cols, rows) = cell.fit(bounds.size);
        let Ok(size) = GridSize::within(cols, rows) else {
            return;
        };
        let first = match area.fit {
            Some(fit) if fit == size => return,
            Some(_) => false,
            None => true,
        };
        area.fit = Some(size);
        if first {
            self.terminal_resize(cx);
            return;
        }
        let Some(opening) = area.shown.clone() else {
            return;
        };
        area.settling = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SETTLE).await;
            let _gone = this.update(cx, |shell, cx| {
                if shell.areas.terminal.owns(&opening) {
                    shell.terminal_resize(cx);
                }
            });
        }));
    }

    fn terminal_resize(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        if let Some(model) = area.model.as_mut() {
            refit(model, area.fit);
        }
        self.terminal_draw(cx);
    }
}
