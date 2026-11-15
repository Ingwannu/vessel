//! The grid, through the model's `replay`: what a program's bytes put on
//! the screen and in history, and the bounds hostile bytes and sizes
//! cannot pass.

mod behaviour;
mod hostile;
mod pen;
mod reflow;
mod wide;

use super::{Cell, CursorCell, Glyph, Grid, Scroll, Wrap};
use crate::terminal::{GridSize, ScrollbackLines, TerminalModel};

/// A model `cols × rows` keeping `scrollback` lines.
fn model(cols: u16, rows: u16, scrollback: u32) -> TerminalModel {
    let size = GridSize::new(cols, rows).unwrap_or_default();
    let scrollback = ScrollbackLines::new(scrollback).unwrap_or_default();
    TerminalModel::new(size, scrollback)
}

/// `bytes` applied to their end.
fn play(model: &mut TerminalModel, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let taken = model.replay(rest).consumed;
        assert!(taken > 0);
        rest = rest.get(taken..).unwrap_or_default();
    }
}

/// A row as text: an unprinted cell is a space, a continuation nothing
/// (its scalar stands for both cells), trailing spaces dropped.
fn text(cells: &[Cell]) -> String {
    let row: String = cells
        .iter()
        .filter_map(|cell| match cell.glyph() {
            Glyph::Empty => Some(' '),
            Glyph::Scalar(c) => Some(c),
            Glyph::Continuation => None,
        })
        .collect();
    row.trim_end().to_owned()
}

/// The live screen, one string a row.
fn screen(model: &TerminalModel) -> Vec<String> {
    model
        .grid()
        .screen_lines()
        .iter()
        .map(|line| text(line.cells()))
        .collect()
}

/// History, oldest first.
fn history(model: &TerminalModel) -> Vec<String> {
    let grid = model.grid();
    (0..grid.history_len())
        .filter_map(|index| grid.history_line(index))
        .map(|line| text(line.cells()))
        .collect()
}

/// Where the cursor is on the live screen, `(row, col)`.
fn cursor(model: &TerminalModel) -> Option<(u16, u16)> {
    model
        .snapshot(Scroll(0))
        .cursor
        .map(|CursorCell { row, col }| (row, col))
}

/// What holds whatever bytes and sizes came before: every row as wide as
/// the grid, as many rows as it has, the cursor on it, history within
/// both bounds, and a snapshot of exactly the grid's cells.
fn check(grid: &Grid) {
    let (cols, rows) = (usize::from(grid.size.cols()), usize::from(grid.size.rows()));
    for screen in [&grid.primary, &grid.alternate] {
        assert_eq!(screen.rows(), rows);
        assert!(screen.lines().iter().all(|line| line.cells().len() == cols));
        assert!(screen.lines().iter().all(|line| paired(line.cells())));
        assert!(screen.region.top < screen.region.bottom || rows == 1);
        assert!(screen.region.bottom < grid.size.rows());
    }
    assert!(grid.at.row < grid.size.rows() && grid.at.col < grid.size.cols());
    assert!(grid.history.cells() <= super::CELL_LIMIT);
    // Reflow keeps history no wider than the grid.
    for index in 0..grid.history.len() {
        let width = grid
            .history
            .line(index)
            .map_or(0, |line| line.cells().len());
        assert!(width <= cols);
    }
    let limit = usize::try_from(grid.history.limit().get()).unwrap_or(usize::MAX);
    assert!(grid.history.len() <= limit);
    let back = grid.snapshot(Scroll(u32::MAX));
    assert_eq!(back.cells.len(), cols.saturating_mul(rows));
    assert!(back.cells.chunks(cols.max(1)).all(paired));
}

/// Every continuation follows a wide scalar, and every wide scalar is
/// followed by its continuation.
fn paired(cells: &[Cell]) -> bool {
    let mut fixed = cells.to_vec();
    let len = fixed.len();
    super::width::fix_pairs(&mut fixed, 0, len, Cell::BLANK);
    fixed == cells
}

/// Whether a line wrapped onto the next.
fn wrap_of(model: &TerminalModel, row: usize) -> Option<Wrap> {
    model.grid().screen_lines().get(row).map(super::Line::wrap)
}
