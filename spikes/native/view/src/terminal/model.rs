//! The terminal model: the decoder and the grid behind one call per read
//! (ADR 0073's contract). `replay` applies bytes that must not be answered:
//! history, a fold of the ring, and every viewer that may not answer. The
//! answered-query slice adds `feed`, which may return reply bytes.

use super::grid::{Grid, Scroll, Snapshot};
use super::parse::{Decoded, Decoder, Source};
use super::size::{GridSize, ScrollbackLines};

/// What one call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feed {
    /// Bytes taken from the front of the slice, at most
    /// [`FEED_BUDGET`](super::parse::FEED_BUDGET).
    /// The caller keeps the rest for the next call.
    pub consumed: usize,
}

/// A terminal's model: the program's byte stream decoded and applied.
#[derive(Debug)]
pub struct TerminalModel {
    decoder: Decoder,
    grid: Grid,
    /// The actions of the call being applied; emptied after each call, so
    /// it holds at most one budget's actions.
    decoded: Vec<Decoded>,
}

impl TerminalModel {
    /// An empty terminal of `size` keeping `scrollback` lines of history.
    #[must_use]
    pub fn new(size: GridSize, scrollback: ScrollbackLines) -> Self {
        Self {
            decoder: Decoder::new(),
            grid: Grid::new(size, scrollback),
            decoded: Vec::new(),
        }
    }

    /// Apply at most [`FEED_BUDGET`](super::parse::FEED_BUDGET) bytes of
    /// `bytes`; nothing in them is answered.
    pub fn replay(&mut self, bytes: &[u8]) -> Feed {
        let consumed = self
            .decoder
            .decode(bytes, Source::Replay, &mut self.decoded);
        for decoded in self.decoded.drain(..) {
            self.grid.apply(&decoded.action);
        }
        Feed { consumed }
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
