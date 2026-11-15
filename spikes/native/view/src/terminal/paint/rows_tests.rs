//! Rows as the grid draws them: runs of like cells, wide scalars alone,
//! blanks joining and never starting a run, fills of one colour.

use super::{Ink, PaintedFill, PaintedRow, Palette, rows};
use crate::terminal::grid::{Rgb, Scroll, Underline, Weight};
use crate::terminal::{GridSize, ScrollbackLines, TerminalModel};

const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}

/// ANSI colour `n` is `(n, n, n)`; foreground 200, background 10.
fn palette() -> Palette {
    let mut ansi = [Rgb::default(); 16];
    for (n, colour) in (0_u8..).zip(ansi.iter_mut()) {
        *colour = rgb(n, n, n);
    }
    Palette {
        ansi,
        foreground: rgb(200, 200, 200),
        background: rgb(10, 10, 10),
        cursor: rgb(1, 2, 3),
    }
}

/// The first row after `bytes` on an 8 × 2 grid.
fn first_row(bytes: &[u8]) -> PaintedRow {
    let size = GridSize::new(8, 2).unwrap_or_default();
    let mut model = TerminalModel::new(size, ScrollbackLines::default());
    let mut rest = bytes;
    while !rest.is_empty() {
        let taken = model.replay(rest).consumed;
        rest = rest.get(taken..).unwrap_or_default();
    }
    rows(&model.snapshot(Scroll(0)), &palette())
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// Each run as `(col, text)`.
fn runs(row: &PaintedRow) -> Vec<(u16, &str)> {
    row.runs
        .iter()
        .map(|run| (run.col, run.text.as_str()))
        .collect()
}

#[test]
fn plain_text_is_one_run_and_blanks_join_it_inside_but_not_at_its_end() {
    let row = first_row(b"a  b   ");
    assert_eq!(runs(&row), [(0, "a  b")]);
    assert!(row.fills.is_empty());
    // A blank never starts a run.
    assert_eq!(runs(&first_row(b"   x")), [(3, "x")]);
}

#[test]
fn a_change_of_look_starts_a_run() {
    let row = first_row(b"a\x1b[31mb\x1b[1mc\x1b[0md");
    assert_eq!(runs(&row), [(0, "a"), (1, "b"), (2, "c"), (3, "d")]);
    assert_eq!(row.runs.get(1).map(|run| run.ink.fg), Some(rgb(1, 1, 1)));
    // Bold red is drawn bright.
    assert_eq!(
        row.runs.get(2).map(|run| (run.ink.fg, run.ink.weight)),
        Some((rgb(9, 9, 9), Weight::Bold))
    );
}

#[test]
fn a_wide_scalar_is_a_run_of_its_own_and_its_fill_covers_both_cells() {
    let row = first_row("a\x1b[44m한\x1b[0mb".as_bytes());
    assert_eq!(runs(&row), [(0, "a"), (1, "한"), (3, "b")]);
    assert_eq!(
        row.fills,
        [PaintedFill {
            col: 1,
            cells: 2,
            color: rgb(4, 4, 4)
        }]
    );
}

#[test]
fn fills_of_one_colour_join_and_the_grids_own_background_is_not_drawn() {
    let row = first_row(b"\x1b[41mab\x1b[42mc\x1b[0md");
    assert_eq!(
        row.fills,
        [
            PaintedFill {
                col: 0,
                cells: 2,
                color: rgb(1, 1, 1)
            },
            PaintedFill {
                col: 2,
                cells: 1,
                color: rgb(2, 2, 2)
            },
        ]
    );
    // An erase in a colour fills the rest of the row.
    let erased = first_row(b"x\x1b[43m\x1b[K");
    assert_eq!(
        erased.fills,
        [PaintedFill {
            col: 1,
            cells: 7,
            color: rgb(3, 3, 3)
        }]
    );
    assert_eq!(runs(&erased), [(0, "x")]);
}

#[test]
fn concealed_text_draws_nothing_but_lined_blanks_draw_their_line() {
    assert!(first_row(b"\x1b[8mab").runs.is_empty());
    let lined = first_row(b"\x1b[4m  \x1b[0mx");
    assert_eq!(runs(&lined), [(0, "  "), (2, "x")]);
    assert_eq!(
        lined.runs.first().map(|run| run.ink.underline),
        Some(Underline::Single)
    );
}

#[test]
fn an_unlined_blank_does_not_take_the_line_of_the_run_before_it() {
    // `\e[4mab\e[0m c` and `\e[9mab\e[0m c`: the line stops under or
    // through "ab"; the plain space shows nothing, so "c" starts its own
    // run (the first capture drew the underline on to the next word).
    for sgr in [b"4", b"9"] {
        let mut bytes = b"\x1b[".to_vec();
        bytes.extend_from_slice(sgr);
        bytes.extend_from_slice(b"mab\x1b[0m c");
        let row = first_row(&bytes);
        assert_eq!(runs(&row), [(0, "ab"), (3, "c")]);
    }
}

#[test]
fn inverse_and_faint_reach_the_run() {
    let inverse = first_row(b"\x1b[7mab");
    assert_eq!(
        inverse.fills,
        [PaintedFill {
            col: 0,
            cells: 2,
            color: rgb(200, 200, 200)
        }]
    );
    assert_eq!(
        inverse.runs.first().map(|run| run.ink.fg),
        Some(rgb(10, 10, 10))
    );
    let faint = first_row(b"\x1b[2mz");
    assert_eq!(faint.runs.first().map(|run| run.ink.ink), Some(Ink::Faint));
}
