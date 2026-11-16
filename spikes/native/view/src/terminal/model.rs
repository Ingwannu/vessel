//! The terminal model: the decoder and the grid behind one call per read
//! (ADR 0073's contract). `replay` applies bytes that must not be answered:
//! history, a fold of the ring, and every viewer whose `query_duty` is
//! `Silent`. `feed` is the same parse and returns the replies of the
//! queries that began in its own bytes.

use super::grid::{Grid, Scroll, Snapshot};
use super::paint::Palette;
use super::parse::{Decoded, Decoder, Source};
use super::reply::{answer, query};
use super::size::{GridSize, ScrollbackLines};

/// What one call did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Feed {
    /// Bytes taken from the front of the slice, at most
    /// [`FEED_BUDGET`](super::parse::FEED_BUDGET).
    /// The caller keeps the rest for the next call.
    pub consumed: usize,
    /// The answers to the queries in those bytes, in order, for the
    /// terminal's input; always empty from `replay`.
    pub replies: Vec<u8>,
}

/// A terminal's model: the program's byte stream decoded and applied.
#[derive(Debug)]
pub struct TerminalModel {
    decoder: Decoder,
    grid: Grid,
    /// The actions of the call being applied; emptied after each call, so
    /// it holds at most one budget's actions.
    decoded: Vec<Decoded>,
    /// The colours the grid is drawn in, for the OSC colour queries.
    palette: Option<Palette>,
}

impl TerminalModel {
    /// An empty terminal of `size` keeping `scrollback` lines of history.
    #[must_use]
    pub fn new(size: GridSize, scrollback: ScrollbackLines) -> Self {
        Self {
            decoder: Decoder::new(),
            grid: Grid::new(size, scrollback),
            decoded: Vec::new(),
            palette: None,
        }
    }

    /// The colours the screen draws in, which OSC 10, 11 and 12 report.
    /// Until they are given, those queries are not answered.
    pub const fn set_palette(&mut self, palette: Palette) {
        self.palette = Some(palette);
    }

    /// Apply at most [`FEED_BUDGET`](super::parse::FEED_BUDGET) bytes of
    /// `bytes`; nothing in them is answered.
    pub fn replay(&mut self, bytes: &[u8]) -> Feed {
        self.apply(bytes, Source::Replay)
    }

    /// Apply at most [`FEED_BUDGET`](super::parse::FEED_BUDGET) bytes of
    /// live output, answering each query of the closed set whose sequence
    /// began in these bytes (one begun under `replay` never answers).
    pub fn feed(&mut self, bytes: &[u8]) -> Feed {
        self.apply(bytes, Source::Live)
    }

    fn apply(&mut self, bytes: &[u8], source: Source) -> Feed {
        let consumed = self.decoder.decode(bytes, source, &mut self.decoded);
        let mut replies = Vec::new();
        for decoded in self.decoded.drain(..) {
            // Answered before the action applies, so a report says where
            // the cursor is when the program asked.
            if decoded.began == Source::Live
                && let Some(query) = query(&decoded.action)
            {
                answer(
                    query,
                    self.grid.cursor(),
                    self.palette.as_ref(),
                    &mut replies,
                );
            }
            self.grid.apply(&decoded.action);
        }
        Feed { consumed, replies }
    }

    /// Change retention without clearing rows that still fit; evicted rows never revive.
    pub fn set_scrollback(&mut self, limit: ScrollbackLines) {
        self.grid.set_scrollback(limit);
    }

    /// Resize the grid.
    pub fn resize(&mut self, size: GridSize) {
        self.grid.resize(size);
    }

    /// The visible cells, `scroll` lines back into history.
    #[must_use]
    pub fn snapshot(&self, scroll: Scroll) -> Snapshot {
        self.grid.snapshot(scroll)
    }

    /// The grid, read-only.
    #[must_use]
    pub const fn grid(&self) -> &Grid {
        &self.grid
    }
}
