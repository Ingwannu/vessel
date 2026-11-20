//! Reflow of soft-wrapped lines when the grid changes width.
//!
//! The alternate screen is not joined into scrollback: a full-screen
//! program must not dump its frame into the history when the window moves.

use super::super::types::{Attrs, Cell, Glyph, GridSize, blank_cells};
use super::{Line, TerminalModel, blank_line};

impl TerminalModel {
    pub(super) fn reflow_primary(&mut self, size: GridSize) {
        let mut logical = Vec::new();
        let mut carry = Vec::new();
        for line in self.history.iter().chain(self.primary.iter()) {
            carry.extend(content(line));
            if !line.wrapped {
                logical.push(std::mem::take(&mut carry));
            }
        }
        if !carry.is_empty() {
            logical.push(carry);
        }
        let cols = usize::from(size.cols());
        let wrapped = logical
            .into_iter()
            .flat_map(|line| wrap_logical(line, cols))
            .collect::<Vec<_>>();
        let rows = usize::from(size.rows());
        let split = wrapped.len().saturating_sub(rows);
        self.history.clear();
        for line in wrapped.iter().take(split).cloned() {
            self.push_history(line);
        }
        self.primary = wrapped.into_iter().skip(split).collect();
        while self.primary.len() < rows {
            self.primary.push(blank_line(size.cols()));
        }
    }
}

/// Clip or pad `lines` to `size`. Wrapped flags are dropped: this is the
/// alternate screen, which does not reflow into scrollback.
pub(super) fn resize_plain(lines: &[Line], size: GridSize) -> Vec<Line> {
    let cols = usize::from(size.cols());
    let rows = usize::from(size.rows());
    let mut out = Vec::new();
    for line in lines.iter().take(rows) {
        let mut cells = blank_cells(cols, Attrs::plain());
        for (index, cell) in line.cells.iter().take(cols).enumerate() {
            if let Some(slot) = cells.get_mut(index) {
                *slot = *cell;
            }
        }
        out.push(Line {
            cells,
            wrapped: false,
        });
    }
    while out.len() < rows {
        out.push(blank_line(size.cols()));
    }
    out
}

fn content(line: &Line) -> Vec<Cell> {
    if line.wrapped {
        return line.cells.clone();
    }
    let mut end = line.cells.len();
    while end > 0 {
        let Some(cell) = line.cells.get(end.saturating_sub(1)) else {
            break;
        };
        if cell.glyph() != Glyph::Empty {
            break;
        }
        end = end.saturating_sub(1);
    }
    line.cells.iter().take(end).copied().collect()
}

fn wrap_logical(cells: Vec<Cell>, cols: usize) -> Vec<Line> {
    if cols == 0 {
        return Vec::new();
    }
    if cells.is_empty() {
        return vec![Line {
            cells: blank_cells(cols, Attrs::plain()),
            wrapped: false,
        }];
    }
    let mut lines = Vec::new();
    let mut row = blank_cells(cols, Attrs::plain());
    let mut col = 0usize;
    let mut index = 0usize;
    while index < cells.len() {
        let span = cluster_span(&cells, index);
        if col.saturating_add(span) > cols {
            lines.push(Line {
                cells: std::mem::replace(&mut row, blank_cells(cols, Attrs::plain())),
                wrapped: true,
            });
            col = 0;
        }
        copy_span(
            &cells,
            &mut row,
            index,
            col,
            span.min(cols.saturating_sub(col)),
        );
        col = col.saturating_add(span.min(cols.saturating_sub(col)));
        index = index.saturating_add(span);
        if col == cols && index < cells.len() {
            lines.push(Line {
                cells: std::mem::replace(&mut row, blank_cells(cols, Attrs::plain())),
                wrapped: true,
            });
            col = 0;
        }
    }
    lines.push(Line {
        cells: row,
        wrapped: false,
    });
    lines
}

fn copy_span(cells: &[Cell], row: &mut [Cell], index: usize, col: usize, take: usize) {
    for step in 0..take {
        if let Some(slot) = row.get_mut(col.saturating_add(step)) {
            if let Some(cell) = cells.get(index.saturating_add(step)) {
                *slot = *cell;
            }
        }
    }
}

fn cluster_span(cells: &[Cell], index: usize) -> usize {
    let Some(cell) = cells.get(index) else {
        return 1;
    };
    if cell.glyph() == Glyph::Continuation {
        return 1;
    }
    if matches!(
        cells.get(index.saturating_add(1)).copied().map(Cell::glyph),
        Some(Glyph::Continuation)
    ) {
        2
    } else {
        1
    }
}
