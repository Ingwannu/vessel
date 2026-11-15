//! A terminal's bounded position in history; the grid remains the only renderer.

use super::grid::{Grid, Scroll, Shown};

/// A viewport never addresses rows older than the grid retains.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Viewport(Scroll);

impl Viewport {
    /// The effective offset, including alternate-screen and history eviction.
    #[must_use]
    pub fn offset(self, grid: &Grid) -> Scroll {
        Scroll(self.0.0.min(maximum(grid).0))
    }

    /// Move toward older output; saturating addition refuses overflow.
    pub fn earlier(&mut self, lines: Scroll, grid: &Grid) {
        self.0 = Scroll(
            self.offset(grid)
                .0
                .saturating_add(lines.0)
                .min(maximum(grid).0),
        );
    }

    /// Move back toward live output; zero is the live screen.
    pub fn later(&mut self, lines: Scroll, grid: &Grid) {
        self.0 = Scroll(self.offset(grid).0.saturating_sub(lines.0));
    }
}

fn maximum(grid: &Grid) -> Scroll {
    match grid.shown() {
        Shown::Primary => Scroll(u32::try_from(grid.history_len()).unwrap_or(u32::MAX)),
        Shown::Alternate => Scroll::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::grid::Glyph;
    use super::*;
    use crate::terminal::{GridSize, ScrollbackLines, TerminalModel};

    #[test]
    fn old_output_is_viewable_and_returning_to_live_restores_the_cursor() {
        let mut model =
            TerminalModel::new(GridSize::new(12, 2).unwrap(), ScrollbackLines::default());
        let feed = model.replay(b"old\r\nsecond\r\nlive\r\nlast");
        assert!(feed.replies.is_empty());
        let mut viewport = Viewport::default();
        viewport.earlier(Scroll(u32::MAX), model.grid());
        let old = model.snapshot(viewport.offset(model.grid()));
        assert_eq!(old.cells.first().unwrap().glyph(), Glyph::Scalar('o'));
        assert!(old.cursor.is_none());
        viewport.later(Scroll(u32::MAX), model.grid());
        assert_eq!(viewport.offset(model.grid()), Scroll::default());
        assert!(
            model
                .snapshot(viewport.offset(model.grid()))
                .cursor
                .is_some()
        );
        let feed = model.replay(b"\x1b[?1049h");
        assert!(feed.replies.is_empty());
        viewport.earlier(Scroll(u32::MAX), model.grid());
        assert_eq!(viewport.offset(model.grid()), Scroll::default());
    }
}
