//! The terminal's grid as a view of its own (the region rule in
//! `crate::region`): it holds the frame it draws and nothing else, and
//! only the terminal area updates and notifies it, so a program's output
//! draws this view again and not the shell's other parts.

use std::rc::Rc;

use vessel_app_kit::terminal_grid::{CellSize, GridFrame, terminal_grid};
use vessel_app_kit::ui::{Context, IntoElement, Pixels, Render, Window};

/// The grid view.
pub(crate) struct TerminalGrid {
    /// What it draws.
    pub(super) frame: Rc<GridFrame>,
    /// One cell's size, measured from the code face.
    pub(super) cell: CellSize,
}

impl TerminalGrid {
    /// An empty grid until the first frame comes. Its cell is zero until
    /// the screen is measured: a frame with no rows and no cursor draws
    /// only its background, which does not use the cell.
    pub(super) fn empty() -> Self {
        Self {
            frame: Rc::new(GridFrame {
                rows: Vec::new(),
                cursor: None,
                background: gpui::transparent_black(),
                font_size: super::font_size(),
            }),
            cell: CellSize {
                width: Pixels::ZERO,
                height: Pixels::ZERO,
            },
        }
    }
}

impl Render for TerminalGrid {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        terminal_grid(Rc::clone(&self.frame), self.cell)
    }
}
