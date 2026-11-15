//! Appearance and viewport identity invalidate a quiet retained frame too.

use std::rc::Rc;
use vessel_app_kit::ui::{Context, Pixels};
use vessel_app_kit::{
    Scheme,
    fonts::{FontStack, Typeface, active},
    terminal_grid::{CellSize, GridCursor, GridFrame},
};
use vessel_app_view::terminal::{
    GridSize,
    grid::{Generation, Scroll},
    paint::rows,
};

use super::{Blink, SETTLE, convert, font_size, replay::refit};
use crate::shell::Shell;

/// Appearance and viewport inputs that can change painted text or backgrounds.
#[derive(Debug, Clone, PartialEq)]
struct RowsKey {
    generation: Generation,
    scheme: Scheme,
    font: Pixels,
    stack: FontStack,
    cell: CellSize,
    scroll: Scroll,
}

/// Cursor phase is deliberately separate from immutable row identity.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DrawKey {
    rows: RowsKey,
    blink: Blink,
}

/// The unblinked frame keeps the model's cursor authoritative in both phases.
pub(super) struct Painted {
    key: RowsKey,
    frame: GridFrame,
}

impl Painted {
    fn new(
        key: RowsKey,
        model: &vessel_app_view::terminal::TerminalModel,
        tokens: &vessel_app_kit::Tokens,
    ) -> Self {
        let palette = convert::palette(&tokens.terminal);
        let snapshot = model.snapshot(key.scroll);
        let font_size = key.font;
        Self {
            key,
            frame: GridFrame {
                rows: rows(&snapshot, &palette).iter().map(convert::row).collect(),
                cursor: snapshot.cursor.map(|cell| GridCursor {
                    row: cell.row,
                    col: cell.col,
                    color: convert::hsla(palette.cursor),
                }),
                background: convert::hsla(palette.background),
                font_size,
            },
        }
    }

    fn frame(&self, blink: Blink) -> GridFrame {
        let mut frame = self.frame.clone();
        if blink == Blink::Off {
            frame.cursor = None;
        }
        frame
    }
}

impl Shell {
    /// Repaint rows only for their own inputs; cursor-only frames share them.
    pub(super) fn terminal_draw(&mut self, cx: &mut Context<'_, Self>) {
        let tokens = vessel_app_kit::tokens(cx);
        let area = &mut self.areas.terminal;
        let Some(model) = area.model.as_ref() else {
            return;
        };
        area.viewport.refresh(model.grid());
        let key = RowsKey {
            generation: model.grid().generation(),
            scheme: tokens.scheme,
            font: font_size(),
            stack: active(Typeface::Code),
            cell: area.grid.read(cx).cell,
            scroll: area.viewport.offset(model.grid()),
        };
        let now = DrawKey {
            rows: key.clone(),
            blink: area.blink,
        };
        if area.drawn.as_ref() == Some(&now) {
            return;
        }
        let painted = area
            .painted
            .get_or_insert_with(|| Painted::new(key.clone(), model, &tokens));
        if painted.key != key {
            *painted = Painted::new(key, model, &tokens);
        }
        let frame = Rc::new(painted.frame(area.blink));
        area.drawn = Some(now);
        area.grid.update(cx, |grid, cx| {
            grid.frame = frame;
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
