//! The grid, through the model's `replay`: what a program's bytes put on
//! the screen and in history, and the bounds hostile bytes and sizes
//! cannot pass.

mod behaviour;
mod hostile;
mod pen;

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

/// A row as text: an unprinted cell is a space, trailing spaces dropped.
fn text(cells: &[Cell]) -> String {
    let row: String = cells
        .iter()
        .map(|cell| match cell.glyph() {
            Glyph::Empty => ' ',
            Glyph::Scalar(c) => c,
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
        assert!(screen.region.top < screen.region.bottom || rows == 1);
        assert!(screen.region.bottom < grid.size.rows());
    }
    assert!(grid.at.row < grid.size.rows() && grid.at.col < grid.size.cols());
    assert!(grid.history.cells() <= super::CELL_LIMIT);
    let limit = usize::try_from(grid.history.limit().get()).unwrap_or(usize::MAX);
    assert!(grid.history.len() <= limit);
    let back = Scroll(u32::MAX);
    assert_eq!(grid.snapshot(back).cells.len(), cols.saturating_mul(rows));
}

/// Whether a line wrapped onto the next.
fn wrap_of(model: &TerminalModel, row: usize) -> Option<Wrap> {
    model.grid().screen_lines().get(row).map(super::Line::wrap)
}
