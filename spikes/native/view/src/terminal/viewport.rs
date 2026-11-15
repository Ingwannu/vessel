//! A terminal's bounded position in history; the grid remains the only renderer.

use super::grid::{Grid, HistoryRow, Scroll, Shown};

/// A viewport pins an address, not a distance from moving live output.
/// Its retained range also accepts gestures while an executor owns the grid.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Viewport {
    anchor: Option<HistoryRow>,
    retained: Option<HistoryRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HistoryRange {
    first: HistoryRow,
    end: HistoryRow,
    shown: Shown,
}

impl HistoryRange {
    fn of(grid: &Grid) -> Self {
        let first = grid.history_first();
        Self {
            first,
            end: first.after(Scroll(
                u32::try_from(grid.history_len()).unwrap_or(u32::MAX),
            )),
            shown: grid.shown(),
        }
    }

    fn offset(self, anchor: Option<HistoryRow>) -> Scroll {
        if self.shown == Shown::Alternate {
            return Scroll::default();
        }
        anchor
            .and_then(|at| at.within(self.first, self.end))
            .map_or(Scroll::default(), |at| self.end.distance(at))
    }
}

impl Viewport {
    /// Observe each returned grid before drawing it. Evicted anchors clamp
    /// to the oldest row; a clear invalidates the old history's addresses.
    pub fn refresh(&mut self, grid: &Grid) {
        let range = HistoryRange::of(grid);
        self.anchor = self
            .anchor
            .and_then(|at| at.within(range.first, range.end))
            .filter(|at| *at != range.end);
        self.retained = Some(range);
    }

    /// Effective offset even if new output has not yet been presented.
    #[must_use]
    pub fn offset(self, grid: &Grid) -> Scroll {
        HistoryRange::of(grid).offset(self.anchor)
    }

    /// Move toward older output using the current grid.
    pub fn earlier(&mut self, lines: Scroll, grid: &Grid) {
        self.refresh(grid);
        self.earlier_retained(lines);
    }

    /// Move toward live output using the current grid.
    pub fn later(&mut self, lines: Scroll, grid: &Grid) {
        self.refresh(grid);
        self.later_retained(lines);
    }

    /// Retain bounded intent against the last presented range while parsing.
    pub fn earlier_retained(&mut self, lines: Scroll) {
        let Some(range) = self.retained else {
            return;
        };
        let back = range
            .offset(self.anchor)
            .0
            .saturating_add(lines.0)
            .min(range.end.distance(range.first).0);
        self.set_back(range, Scroll(back));
    }

    /// Returning to zero follows output that arrives during the handover too.
    pub fn later_retained(&mut self, lines: Scroll) {
        let Some(range) = self.retained else {
            return;
        };
        let back = range.offset(self.anchor).0.saturating_sub(lines.0);
        self.set_back(range, Scroll(back));
    }

    fn set_back(&mut self, range: HistoryRange, back: Scroll) {
        if range.shown == Shown::Primary {
            self.anchor = (back.0 != 0).then(|| range.end.before(back));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::grid::Glyph;
    use super::*;
    use crate::terminal::{GridSize, ScrollbackLines, TerminalModel};

    fn first(model: &TerminalModel, viewport: Viewport) -> char {
        match model
            .snapshot(viewport.offset(model.grid()))
            .cells
            .first()
            .unwrap()
            .glyph()
        {
            Glyph::Scalar(ch) => ch,
            other @ (Glyph::Empty | Glyph::Continuation) => {
                panic!("expected a row label: {other:?}");
            }
        }
    }

    #[test]
    fn inspected_row_stays_until_eviction_then_clamps_and_live_follows() {
        let mut model = TerminalModel::new(
            GridSize::new(12, 2).unwrap(),
            ScrollbackLines::new(4).unwrap(),
        );
        let _feed = model.replay(b"0\r\n1\r\n2\r\n3\r\n4");
        let mut viewport = Viewport::default();
        viewport.earlier(Scroll(1), model.grid());
        assert_eq!(first(&model, viewport), '2');
        let _feed = model.replay(b"\r\n5");
        assert_eq!(first(&model, viewport), '2');
        let _feed = model.replay(b"\r\n6");
        assert_eq!(first(&model, viewport), '2');
        // The cap is full: stable length must not hide the eviction.
        let _feed = model.replay(b"\r\n7");
        assert_eq!(model.grid().history_len(), 4);
        assert_eq!(first(&model, viewport), '2');
        let _feed = model.replay(b"\r\n8");
        assert_eq!(first(&model, viewport), '3');
        viewport.later(Scroll(u32::MAX), model.grid());
        assert_eq!(viewport.offset(model.grid()), Scroll(0));
        let _feed = model.replay(b"\r\n9");
        assert_eq!(first(&model, viewport), '8');
    }

    #[test]
    fn alternate_output_never_moves_the_primary_history_anchor() {
        let mut model =
            TerminalModel::new(GridSize::new(12, 2).unwrap(), ScrollbackLines::default());
        let _feed = model.replay(b"0\r\n1\r\n2\r\n3\r\n4\r\n5");
        let mut viewport = Viewport::default();
        viewport.earlier(Scroll(2), model.grid());
        let _feed = model.replay(b"\x1b[?1049hA\r\nB\r\nC");
        viewport.earlier(Scroll(u32::MAX), model.grid());
        assert_eq!(viewport.offset(model.grid()), Scroll(0));
        let _feed = model.replay(b"\x1b[?1049l\r\n6");
        assert_eq!(first(&model, viewport), '2');
    }

    #[test]
    fn erased_and_reset_history_cannot_revive_a_prior_anchor() {
        for reset in [&b"\x1b[3J"[..], &b"\x1bc"[..]] {
            let mut model =
                TerminalModel::new(GridSize::new(12, 2).unwrap(), ScrollbackLines::default());
            let _feed = model.replay(b"0\r\n1\r\n2\r\n3\r\n4");
            let mut viewport = Viewport::default();
            viewport.earlier(Scroll(2), model.grid());
            let _feed = model.replay(reset);
            let _feed = model.replay(b"\r\na\r\nb\r\nc\r\nd");
            assert_eq!(viewport.offset(model.grid()), Scroll(0));
        }
    }

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
