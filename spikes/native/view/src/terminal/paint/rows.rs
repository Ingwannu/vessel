//! A snapshot's rows as the kit's grid draws them: runs of cells that
//! look the same, and the backgrounds under them, in drawn colours (ADR
//! 0073, T1). The kit receives colours and places; every rule about which
//! cell joins which run is here, with tests.
//!
//! [Decision Log]
//! - Purpose: the grid control shapes one run at a time and places it at
//!   its first cell; fewer runs is less shaping, and a wrong join puts a
//!   glyph on the wrong cell.
//! - Rules: a wide scalar is a run of its own (its fallback advance is
//!   not two cells, so it must start on its own cell); a continuation is
//!   skipped (its scalar covers it, and its background is the scalar's
//!   second cell); an unprinted, concealed or space cell with no line
//!   under or through it shows nothing, so it joins the run it follows
//!   as a space when that run draws no line either (a lined run would
//!   draw its line over the blank: the first capture showed an underline
//!   running on to the next word) and never starts one; runs lose
//!   trailing spaces. A background
//!   is a fill of adjacent cells of one colour; the palette's own
//!   background is not drawn (the grid already is).

use super::{Ink, Painted, Palette, paint};
use crate::terminal::grid::{Cell, Glyph, Rgb, Slant, Snapshot, Strike, Underline, Weight};

/// How a run's glyphs are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunInk {
    /// The glyphs' colour.
    pub fg: Rgb,
    /// Full or faint (a hidden glyph is never in a run).
    pub ink: Ink,
    /// Weight.
    pub weight: Weight,
    /// Slant.
    pub slant: Slant,
    /// The line under the glyphs.
    pub underline: Underline,
    /// The line through them.
    pub strike: Strike,
}

/// Cells from `col` that look the same; one scalar a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintedRun {
    /// The first cell.
    pub col: u16,
    /// The text.
    pub text: String,
    /// How it is drawn.
    pub ink: RunInk,
}

/// A background over `cells` cells from `col`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintedFill {
    /// The first cell.
    pub col: u16,
    /// How many cells.
    pub cells: u16,
    /// The colour.
    pub color: Rgb,
}

/// One row as drawn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaintedRow {
    /// Backgrounds other than the grid's own.
    pub fills: Vec<PaintedFill>,
    /// Text.
    pub runs: Vec<PaintedRun>,
}

/// `snapshot`'s rows in `palette`.
#[must_use]
pub fn rows(snapshot: &Snapshot, palette: &Palette) -> Vec<PaintedRow> {
    let cols = usize::from(snapshot.size.cols());
    snapshot
        .cells
        .chunks(cols.max(1))
        .map(|cells| row(cells, palette))
        .collect()
}

/// One row's cells as runs and fills.
fn row(cells: &[Cell], palette: &Palette) -> PaintedRow {
    let mut out = PaintedRow::default();
    let mut open: Option<PaintedRun> = None;
    for (col, cell) in (0_u16..).zip(cells) {
        let wide = matches!(
            cells
                .get(usize::from(col).saturating_add(1))
                .map(|next| next.glyph()),
            Some(Glyph::Continuation)
        );
        let painted = paint(cell.pen(), palette);
        let span = if wide { 2 } else { 1 };
        match cell.glyph() {
            // Its scalar covers it.
            Glyph::Continuation => {}
            Glyph::Scalar(c) if painted.ink != Ink::Hidden && c != ' ' => {
                fill(&mut out, col, span, painted.bg);
                let ink = run_ink(cell, painted);
                if wide {
                    close(&mut out, open.take());
                    out.runs.push(PaintedRun {
                        col,
                        text: c.to_string(),
                        ink,
                    });
                } else {
                    open = Some(extend(&mut out, open.take(), col, c, ink));
                }
            }
            Glyph::Scalar(_) | Glyph::Empty => {
                fill(&mut out, col, span, painted.bg);
                open = blank(&mut out, open.take(), col, run_ink(cell, painted));
            }
        }
    }
    close(&mut out, open);
    out
}

/// `c` at `col` added to the open run when it looks the same and follows
/// it, else a new run.
fn extend(
    out: &mut PaintedRow,
    open: Option<PaintedRun>,
    col: u16,
    c: char,
    ink: RunInk,
) -> PaintedRun {
    match open {
        Some(mut run) if run.ink == ink && follows(&run, col) => {
            run.text.push(c);
            run
        }
        other => {
            close(out, other);
            PaintedRun {
                col,
                text: c.to_string(),
                ink,
            }
        }
    }
}

/// A cell that shows no glyph: a space in the open run when it follows
/// it and the run draws no line, unless a line is drawn under or through
/// the cell itself (then it is text of its own look).
fn blank(
    out: &mut PaintedRow,
    open: Option<PaintedRun>,
    col: u16,
    ink: RunInk,
) -> Option<PaintedRun> {
    if lined(ink) {
        return Some(extend(out, open, col, ' ', ink));
    }
    match open {
        Some(mut run) if follows(&run, col) && !lined(run.ink) => {
            run.text.push(' ');
            Some(run)
        }
        other => {
            close(out, other);
            None
        }
    }
}

/// Whether a line is drawn under or through glyphs of `ink`.
fn lined(ink: RunInk) -> bool {
    ink.underline != Underline::None || ink.strike != Strike::None
}

/// Whether a cell at `col` is the next after `run`.
fn follows(run: &PaintedRun, col: u16) -> bool {
    let cells = u16::try_from(run.text.chars().count()).unwrap_or(u16::MAX);
    run.col.saturating_add(cells) == col
}

/// The run, without its trailing spaces unless they are lined, kept when
/// anything is left.
fn close(out: &mut PaintedRow, run: Option<PaintedRun>) {
    let Some(mut run) = run else {
        return;
    };
    if !lined(run.ink) {
        let kept = run.text.trim_end_matches(' ').len();
        run.text.truncate(kept);
    }
    if !run.text.is_empty() {
        out.runs.push(run);
    }
}

/// A background of `cells` cells at `col`, joined to the last when it
/// is the same colour and follows it.
fn fill(out: &mut PaintedRow, col: u16, cells: u16, bg: Option<Rgb>) {
    let Some(color) = bg else {
        return;
    };
    if let Some(last) = out.fills.last_mut()
        && last.color == color
        && last.col.saturating_add(last.cells) == col
    {
        last.cells = last.cells.saturating_add(cells);
        return;
    }
    out.fills.push(PaintedFill { col, cells, color });
}

/// A cell's run look.
const fn run_ink(cell: &Cell, painted: Painted) -> RunInk {
    let pen = cell.pen();
    RunInk {
        fg: painted.fg,
        ink: painted.ink,
        weight: pen.weight,
        slant: pen.slant,
        underline: pen.underline,
        strike: pen.strike,
    }
}
