//! Reflow: on a width change, the primary screen's and history's
//! soft-wrapped lines are joined into the lines the program wrote and cut
//! again at the new width (xterm.js reflows its normal buffer the same
//! way; the alternate screen is only cut or padded).
//!
//! [Decision Log]
//! - Purpose: a shell's output stays readable after the window is made
//!   narrower or wider; without reflow a narrower window cuts every long
//!   line and a wider one never takes the cut part back.
//! - Constraint: a wide scalar and its continuation stay on one line; the
//!   cursor stays on the character it was on; history keeps its bounds.
//! - Choice: one pass over every line, oldest first. A line that ended
//!   soft is joined to the next (its trailing unprinted cells dropped: the
//!   one a wide scalar left when it wrapped). Each joined line is cut into
//!   rows of the new width, a row ending one cell early rather than split a
//!   pair (on a one-column grid the pair becomes blank, since it cannot be
//!   drawn). The cursor is carried as an offset into its joined line.
//! - Cost: a reflow touches all of history (at most 4 Mi cells); the
//!   screen asks for a size only after 100 ms without change.

use super::cell::{Cell, Glyph, Line, Wrap};

/// Where the cursor is: a line of the sequence given, and a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Spot {
    pub(super) line: usize,
    pub(super) col: usize,
}

/// `lines` (oldest first) reflowed to `cols`, and where `cursor` lands.
pub(super) fn reflow(
    lines: Vec<Line>,
    cursor: Option<Spot>,
    cols: u16,
) -> (Vec<Line>, Option<Spot>) {
    let mut out = Vec::with_capacity(lines.len());
    let mut joined: Vec<Cell> = Vec::new();
    let mut offset = None;
    let mut moved = None;
    for (index, line) in lines.into_iter().enumerate() {
        let wrap = line.wrap();
        let mut cells = line.into_cells();
        if let Some(cursor) = cursor
            && cursor.line == index
        {
            offset = Some(joined.len().saturating_add(cursor.col));
        }
        if wrap == Wrap::Soft {
            trim(&mut cells);
        }
        joined.append(&mut cells);
        if wrap == Wrap::Hard {
            cut(&mut joined, offset.take(), cols, &mut out, &mut moved);
        }
    }
    // A soft-wrapped line at the very end (nothing printed after the wrap).
    if !joined.is_empty() || offset.is_some() {
        cut(&mut joined, offset, cols, &mut out, &mut moved);
    }
    (out, moved)
}

/// Without trailing unprinted cells.
fn trim(cells: &mut Vec<Cell>) {
    let kept = cells
        .iter()
        .rposition(|cell| *cell != Cell::BLANK)
        .map_or(0, |last| last.saturating_add(1));
    cells.truncate(kept);
}

/// Cut `joined` into rows of `cols` onto `out`, emptying it; `offset` is
/// the cursor's place in it, whose row and column go to `moved`.
fn cut(
    joined: &mut Vec<Cell>,
    offset: Option<usize>,
    cols: u16,
    out: &mut Vec<Line>,
    moved: &mut Option<Spot>,
) {
    trim(joined);
    let width = usize::from(cols);
    if width < 2 {
        blank_pairs(joined);
    }
    let first = out.len();
    let mut starts = Vec::new();
    let mut start = 0;
    loop {
        starts.push(start);
        let mut take = joined.len().saturating_sub(start).min(width);
        let after = start.saturating_add(take);
        // The row's last cell is a wide scalar whose continuation would
        // start the next row: end the row a cell early.
        if take == width
            && take > 1
            && matches!(
                joined.get(after).map(|cell| cell.glyph()),
                Some(Glyph::Continuation)
            )
        {
            take = take.saturating_sub(1);
        }
        let end = start.saturating_add(take);
        let mut row = joined.get(start..end).unwrap_or_default().to_vec();
        row.resize(width, Cell::BLANK);
        out.push(Line::from_cells(row, Wrap::Hard));
        start = end;
        if start >= joined.len() {
            break;
        }
        mark_soft(out);
    }
    if let Some(offset) = offset {
        let index = starts
            .iter()
            .rposition(|start| *start <= offset)
            .unwrap_or(0);
        let row_start = starts.get(index).copied().unwrap_or(0);
        let mut line = first.saturating_add(index);
        let mut col = offset.saturating_sub(row_start);
        // A cursor past the text (spaces not yet printed) may need rows of
        // its own.
        while col >= width && width > 0 {
            mark_soft(out);
            out.push(Line::from_cells(vec![Cell::BLANK; width], Wrap::Hard));
            line = line.saturating_add(1);
            col = col.saturating_sub(width);
        }
        *moved = Some(Spot { line, col });
    }
    joined.clear();
}

/// The last row of `out` continues onto the next.
fn mark_soft(out: &mut [Line]) {
    if let Some(last) = out.last_mut() {
        last.set_wrap(Wrap::Soft);
    }
}

/// Every wide pair blank (a one-column grid cannot draw one).
fn blank_pairs(cells: &mut [Cell]) {
    let len = cells.len();
    for at in 0..len {
        if matches!(
            cells.get(at).map(|cell| cell.glyph()),
            Some(Glyph::Continuation)
        ) {
            if let Some(base) = at.checked_sub(1).and_then(|before| cells.get_mut(before)) {
                *base = Cell::BLANK;
            }
            if let Some(cell) = cells.get_mut(at) {
                *cell = Cell::BLANK;
            }
        }
    }
}
