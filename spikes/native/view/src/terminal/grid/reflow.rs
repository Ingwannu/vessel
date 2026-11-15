//! Reflow: on a width change, the primary screen's and history's
//! soft-wrapped lines are joined into the lines the program wrote and cut
//! again at the new width, as xterm.js reflows its normal buffer (the
//! alternate screen is only cut or padded).
//!
//! [Decision Log]
//! - Purpose: a shell's output stays readable after the window is made
//!   narrower or wider; without reflow a narrower window cuts every long
//!   line and a wider one never takes the cut part back.
//! - Reference: xterm.js 6.1.0-beta.336, the version Paseo pins, with
//!   Paseo's options (`terminal-emulator-runtime.ts` 555–571).
//!   - The run of lines holding the cursor is not reflowed: Paseo leaves
//!     `reflowCursorLine` at its default, false ("shells usually handle
//!     this themselves", `xterm.d.ts` 226–232; `Buffer.ts` 391–398,
//!     `BufferReflow.ts` 45). Its lines are only cut to the new width and
//!     the cursor keeps its row and column; the shell redraws its line
//!     after the resize, and reflowing it too would add a second move to
//!     the shell's own (duplicated or misplaced prompt rows).
//!   - A soft-wrapped row gives up at most one trailing cell when joined:
//!     the blank a wide scalar left when it wrapped, if the next row
//!     starts with a wide scalar (`getWrappedLineTrimmedLength`,
//!     `BufferReflow.ts` 215–229). A gap erased inside a wrapped line
//!     (EL, ECH, DCH) stays. The last row of a run is trimmed.
//! - Choice: one pass over every line, oldest first; each joined line is
//!   cut into rows of the new width, a row ending one cell early rather
//!   than split a pair (on a one-column grid the pair becomes blank, since
//!   it cannot be drawn).
//! - Memory: rows leave this pass unpadded; the caller pads only the rows
//!   the screen shows. The peak is history's own cells, one joined line
//!   and one screen, never lines × width (a test reflows 100,000 empty
//!   lines to 1,024 columns and counts the cells).
//! - Cost: a reflow touches all of history; the screen asks for a size
//!   only after 100 ms without change.

use std::ops::RangeInclusive;

use super::cell::{Cell, Glyph, Line, Wrap};
use super::width::{Width, width};

/// `lines` (oldest first) reflowed to `cols`, and the cursor's line
/// (`cursor`, an index into `lines`) as an index into the result. Rows
/// are not padded to `cols`.
pub(super) fn reflow(
    lines: Vec<Line>,
    cursor: Option<usize>,
    cols: u16,
) -> (Vec<Line>, Option<usize>) {
    let held = cursor.map(|line| run_around(&lines, line));
    let mut out = Vec::with_capacity(lines.len());
    let mut joined: Vec<Cell> = Vec::new();
    let mut moved = None;
    let mut lines = lines.into_iter().enumerate().peekable();
    while let Some((index, line)) = lines.next() {
        if held.as_ref().is_some_and(|held| held.contains(&index)) {
            if cursor == Some(index) {
                moved = Some(out.len());
            }
            out.push(line.cut(cols, Cell::BLANK));
            continue;
        }
        let wrap = line.wrap();
        let mut cells = line.into_cells();
        if wrap == Wrap::Soft
            && cells.last() == Some(&Cell::BLANK)
            && lines.peek().is_some_and(|(_, next)| starts_wide(next))
        {
            cells.truncate(cells.len().saturating_sub(1));
        }
        joined.append(&mut cells);
        if wrap == Wrap::Hard {
            cut(&mut joined, cols, &mut out);
        }
    }
    // A run still open at the very end (its next line was scrolled away).
    if !joined.is_empty() {
        cut(&mut joined, cols, &mut out);
    }
    (out, moved)
}

/// The run of soft-wrapped lines that holds line `at`: from the line
/// after the last hard ending before it to the first hard ending from it.
fn run_around(lines: &[Line], at: usize) -> RangeInclusive<usize> {
    let before = lines.get(..at).unwrap_or_default();
    let start = before
        .iter()
        .rposition(|line| line.wrap() == Wrap::Hard)
        .map_or(0, |hard| hard.saturating_add(1));
    let end = lines
        .get(at..)
        .unwrap_or_default()
        .iter()
        .position(|line| line.wrap() == Wrap::Hard)
        .map_or(lines.len().saturating_sub(1), |hard| {
            at.saturating_add(hard)
        });
    start..=end
}

/// Whether `line` starts with a wide scalar.
fn starts_wide(line: &Line) -> bool {
    matches!(line.cells().first().map(|cell| cell.glyph()), Some(Glyph::Scalar(c)) if width(c) == Width::Two)
}

/// Cut `joined` into rows of at most `cols` onto `out`, emptying it.
fn cut(joined: &mut Vec<Cell>, cols: u16, out: &mut Vec<Line>) {
    let kept = joined
        .iter()
        .rposition(|cell| *cell != Cell::BLANK)
        .map_or(0, |last| last.saturating_add(1));
    joined.truncate(kept);
    let width = usize::from(cols);
    if width < 2 {
        blank_pairs(joined);
    }
    let mut start = 0;
    loop {
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
        let row = joined.get(start..end).unwrap_or_default().to_vec();
        out.push(Line::from_cells(row, Wrap::Hard));
        start = end;
        if start >= joined.len() {
            break;
        }
        if let Some(last) = out.last_mut() {
            last.set_wrap(Wrap::Soft);
        }
    }
    joined.clear();
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
