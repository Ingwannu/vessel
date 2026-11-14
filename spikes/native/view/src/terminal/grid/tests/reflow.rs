//! Reflow on a width change, as xterm.js does it with Paseo's options:
//! soft-wrapped lines rejoin and split again, history with them, a wide
//! pair never split, an erased gap kept, the cursor's own run only cut;
//! the alternate screen only cut or padded; the peak memory bounded.

use super::{check, cursor, history, model, play, screen, wrap_of};
use crate::terminal::grid::reflow::reflow;
use crate::terminal::grid::{Cell, Line, Pen, Wrap};
use crate::terminal::{GridSize, TerminalModel};

fn resize(term: &mut TerminalModel, cols: u16, rows: u16) {
    let size = GridSize::new(cols, rows);
    assert!(size.is_ok());
    if let Ok(size) = size {
        term.resize(size);
    }
    check(term.grid());
}

#[test]
fn a_soft_wrapped_line_rejoins_when_the_grid_widens() {
    let mut term = model(5, 3, 10);
    play(&mut term, b"abcdefgh\r\n");
    assert_eq!(screen(&term), ["abcde", "fgh", ""]);
    resize(&mut term, 10, 3);
    assert_eq!(screen(&term), ["abcdefgh", "", ""]);
    assert_eq!(wrap_of(&term, 0), Some(Wrap::Hard));
    assert_eq!(cursor(&term), Some((1, 0)));
}

#[test]
fn a_long_line_splits_when_the_grid_narrows_and_hard_lines_stay_apart() {
    let mut term = model(10, 4, 10);
    play(&mut term, b"abcdefgh\r\nxy");
    resize(&mut term, 4, 4);
    assert_eq!(screen(&term), ["abcd", "efgh", "xy", ""]);
    assert_eq!(wrap_of(&term, 0), Some(Wrap::Soft));
    assert_eq!(wrap_of(&term, 1), Some(Wrap::Hard));
    assert_eq!(cursor(&term), Some((2, 2)));
}

#[test]
fn history_reflows_with_the_screen() {
    let mut term = model(4, 2, 10);
    play(&mut term, b"abcdefgh\r\nxy");
    assert_eq!(history(&term), ["abcd"]);
    assert_eq!(screen(&term), ["efgh", "xy"]);
    resize(&mut term, 8, 2);
    assert!(history(&term).is_empty());
    assert_eq!(screen(&term), ["abcdefgh", "xy"]);
    assert_eq!(cursor(&term), Some((1, 2)));
    resize(&mut term, 4, 2);
    assert_eq!(history(&term), ["abcd"]);
    assert_eq!(screen(&term), ["efgh", "xy"]);
    assert_eq!(cursor(&term), Some((1, 2)));
}

#[test]
fn a_wide_scalar_moves_whole_to_the_next_row_and_back() {
    let mut term = model(6, 4, 0);
    play(&mut term, "ab한cd\r\n".as_bytes());
    resize(&mut term, 3, 4);
    assert_eq!(screen(&term), ["ab", "한c", "d", ""]);
    resize(&mut term, 6, 4);
    assert_eq!(screen(&term), ["ab한cd", "", "", ""]);
}

#[test]
fn the_cursors_wrapped_line_is_cut_not_reflowed() {
    // A prompt line wrapped once, the cursor at its end: the shell redraws
    // it after the resize, so the grid only cuts or pads it.
    let mut term = model(5, 3, 0);
    play(&mut term, b"$ abcdefgh");
    assert_eq!(screen(&term), ["$ abc", "defgh", ""]);
    resize(&mut term, 4, 3);
    assert_eq!(screen(&term), ["$ ab", "defg", ""]);
    assert_eq!(cursor(&term), Some((1, 3)));
    resize(&mut term, 8, 3);
    assert_eq!(screen(&term), ["$ ab", "defg", ""]);
    // The wrap that was pending on the last column becomes the next cell.
    assert_eq!(cursor(&term), Some((1, 4)));
}

#[test]
fn a_gap_erased_inside_a_wrapped_line_stays() {
    let mut term = model(6, 3, 0);
    play(&mut term, b"abcdefgh\x1b[1;3H\x1b[K\x1b[3;1H");
    assert_eq!(screen(&term), ["ab", "gh", ""]);
    assert_eq!(wrap_of(&term, 0), Some(Wrap::Soft));
    resize(&mut term, 10, 3);
    assert_eq!(screen(&term), ["ab    gh", "", ""]);
}

#[test]
fn blank_rows_below_the_cursor_go_before_any_line_leaves_for_history() {
    let mut term = model(6, 3, 10);
    play(&mut term, b"abcdef\r\n");
    resize(&mut term, 3, 3);
    assert_eq!(screen(&term), ["abc", "def", ""]);
    assert!(history(&term).is_empty());
    assert_eq!(cursor(&term), Some((2, 0)));
}

#[test]
fn a_pending_wrap_becomes_the_cell_after_its_character_when_there_is_room() {
    let mut term = model(3, 2, 0);
    play(&mut term, b"abc");
    resize(&mut term, 6, 2);
    assert_eq!(cursor(&term), Some((0, 3)));
    play(&mut term, b"d");
    assert_eq!(screen(&term), ["abcd", ""]);
}

#[test]
fn the_alternate_screen_is_cut_not_reflowed_and_the_primary_reflows_beneath_it() {
    let mut term = model(3, 2, 10);
    play(&mut term, b"abcdef\x1b[?1049h\x1b[Hxyz12");
    assert_eq!(screen(&term), ["xyz", "12"]);
    resize(&mut term, 6, 2);
    assert_eq!(screen(&term), ["xyz", "12"]);
    play(&mut term, b"\x1b[?1049l");
    assert_eq!(screen(&term), ["abcdef", ""]);
}

#[test]
fn a_one_column_grid_blanks_wide_pairs_and_keeps_its_bounds() {
    let mut term = model(4, 2, 3);
    play(&mut term, "a한b\r\n".as_bytes());
    resize(&mut term, 1, 2);
    resize(&mut term, 5, 2);
    assert!(screen(&term).iter().all(|row| !row.contains('한')));
}

/// Cells held by `lines`.
fn cells(lines: &[Line]) -> usize {
    lines.iter().map(|line| line.cells().len()).sum()
}

#[test]
fn a_reflow_returns_no_more_cells_than_it_was_given_whatever_the_width() {
    // 100,000 empty lines and as many short ones, widened to the most
    // columns: padding every row would be 4 Gi cells.
    let empty = Line::from_cells(Vec::new(), Wrap::Hard);
    let short = Line::from_cells(vec![Cell::printed('x', Pen::PLAIN); 3], Wrap::Hard);
    let mut lines = vec![empty; 100_000];
    lines.extend(std::iter::repeat_n(short, 100_000));
    let given = cells(&lines);
    let (out, _) = reflow(lines, None, GridSize::MAX_COLS);
    assert_eq!(out.len(), 200_000);
    assert!(cells(&out) <= given);
    // And narrowed, nothing is added either.
    let (narrow, _) = reflow(out, None, 2);
    assert!(cells(&narrow) <= given);
}
