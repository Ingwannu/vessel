//! The terminal's grid as a view of its own (the region rule in
//! `crate::region`): it holds the frame it draws and nothing else, and
//! only the terminal area updates and notifies it, so a program's output
//! draws this view again and not the shell's other parts.

use std::rc::Rc;

use vessel_app_kit::terminal_grid::{CellSize, GridFrame, RetainedGrid};
use vessel_app_kit::ui::{Context, IntoElement, Pixels, Render, Window};

/// The grid view.
pub(crate) struct TerminalGrid {
    /// What it draws.
    pub(super) frame: Rc<GridFrame>,
    /// One cell's size, measured from the code face.
    pub(super) cell: CellSize,
    retained: RetainedGrid,
}

impl TerminalGrid {
    /// An empty grid until the first frame comes. Its cell is zero until
    /// the screen is measured: a frame with no rows and no cursor draws
    /// only its background, which does not use the cell.
    pub(super) fn empty(cx: &mut Context<'_, Self>) -> Self {
        let frame = Rc::new(GridFrame {
            rows: Rc::from([]),
            cursor: None,
            background: gpui::transparent_black(),
            font_size: super::font_size(),
        });
        let cell = CellSize {
            width: Pixels::ZERO,
            height: Pixels::ZERO,
        };
        let retained = RetainedGrid::new(&frame, cell, cx);
        Self {
            frame,
            cell,
            retained,
        }
    }
}

impl Render for TerminalGrid {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.retained.draw(Rc::clone(&self.frame), self.cell, cx)
    }
}
