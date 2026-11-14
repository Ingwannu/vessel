//! Reflow on a width change: soft-wrapped lines rejoin and split again,
//! history with them, a wide pair never split, the cursor on its
//! character; the alternate screen only cut or padded.

use super::{check, cursor, history, model, play, screen, wrap_of};
use crate::terminal::grid::Wrap;
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
    play(&mut term, b"abcdefgh");
    assert_eq!(screen(&term), ["abcde", "fgh", ""]);
    resize(&mut term, 10, 3);
    assert_eq!(screen(&term), ["abcdefgh", "", ""]);
    assert_eq!(wrap_of(&term, 0), Some(Wrap::Hard));
    assert_eq!(cursor(&term), Some((0, 8)));
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
fn a_wide_scalar_moves_whole_to_the_next_row() {
    let mut term = model(6, 3, 0);
    play(&mut term, "ab한cd".as_bytes());
    resize(&mut term, 3, 3);
    assert_eq!(screen(&term), ["ab", "한c", "d"]);
    resize(&mut term, 6, 3);
    assert_eq!(screen(&term), ["ab한cd", "", ""]);
}

#[test]
fn blank_rows_below_the_cursor_go_before_any_line_leaves_for_history() {
    let mut term = model(6, 3, 10);
    play(&mut term, b"abcdef");
    resize(&mut term, 3, 3);
    assert_eq!(screen(&term), ["abc", "def", ""]);
    assert!(history(&term).is_empty());
    // The wrap stays pending on the new last column.
    assert_eq!(cursor(&term), Some((1, 2)));
    play(&mut term, b"g");
    assert_eq!(screen(&term), ["abc", "def", "g"]);
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
    play(&mut term, "a한b".as_bytes());
    resize(&mut term, 1, 2);
    resize(&mut term, 5, 2);
    assert!(screen(&term).iter().all(|row| !row.contains('한')));
}
