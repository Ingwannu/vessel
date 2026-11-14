//! What the sequences do, each against xterm.js's behaviour where VT100
//! leaves room (ADR 0073: Paseo draws with xterm.js).

use super::{cursor, history, model, play, screen, wrap_of};
use crate::terminal::grid::{Scroll, Shown, Wrap};
use crate::terminal::{GridSize, TerminalModel};

#[test]
fn a_print_at_the_last_column_wraps_only_when_the_next_character_comes() {
    let mut term = model(5, 3, 10);
    play(&mut term, b"abcde");
    assert_eq!(cursor(&term), Some((0, 4)));
    assert_eq!(wrap_of(&term, 0), Some(Wrap::Hard));
    play(&mut term, b"f");
    assert_eq!(screen(&term), ["abcde", "f", ""]);
    assert_eq!(wrap_of(&term, 0), Some(Wrap::Soft));
    assert_eq!(cursor(&term), Some((1, 1)));
    // A carriage return at the last column cancels the wrap.
    play(&mut term, b"\x1b[3;1Hvwxyz\rq");
    assert_eq!(screen(&term), ["abcde", "f", "qwxyz"]);
}

#[test]
fn without_auto_wrap_the_last_column_is_overwritten() {
    let mut term = model(4, 2, 10);
    play(&mut term, b"\x1b[?7labcdef");
    assert_eq!(screen(&term), ["abcf", ""]);
    assert_eq!(cursor(&term), Some((0, 3)));
}

#[test]
fn a_line_feed_at_the_bottom_keeps_the_top_line_in_history() {
    let mut term = model(4, 2, 10);
    play(&mut term, b"one\r\ntwo\r\nsix\r\nten");
    assert_eq!(screen(&term), ["six", "ten"]);
    assert_eq!(history(&term), ["one", "two"]);
}

#[test]
fn su_and_a_region_below_the_top_keep_no_history() {
    let mut term = model(4, 3, 10);
    play(&mut term, b"a\r\nb\r\nc\x1b[S");
    assert_eq!(screen(&term), ["b", "c", ""]);
    assert!(history(&term).is_empty());
    // A region from row 2: its line feeds scroll it alone.
    play(&mut term, b"\x1b[2;3r\x1b[3;1Hx\ny\nz");
    assert_eq!(screen(&term), ["b", " y", "  z"]);
    assert!(history(&term).is_empty());
}

#[test]
fn the_alternate_screen_starts_clear_keeps_no_history_and_leaves_the_primary_as_it_was() {
    let mut term = model(6, 2, 10);
    play(&mut term, b"shell\r\n$ ");
    play(&mut term, b"\x1b[?1049h");
    assert_eq!(term.grid().shown(), Shown::Alternate);
    assert_eq!(screen(&term), ["", ""]);
    play(&mut term, b"\x1b[Hvim\r\n~\r\n~\r\n~");
    assert!(history(&term).is_empty());
    play(&mut term, b"\x1b[?1049l");
    assert_eq!(term.grid().shown(), Shown::Primary);
    assert_eq!(screen(&term), ["shell", "$"]);
    assert_eq!(cursor(&term), Some((1, 2)));
    // Entering again clears what the last visit left.
    play(&mut term, b"\x1b[?1049h");
    assert_eq!(screen(&term), ["", ""]);
}

#[test]
fn origin_mode_counts_rows_from_the_region_and_keeps_the_cursor_in_it() {
    let mut term = model(4, 5, 10);
    play(&mut term, b"\x1b[2;4r\x1b[?6h");
    assert_eq!(cursor(&term), Some((1, 0)));
    play(&mut term, b"\x1b[2;2H");
    assert_eq!(cursor(&term), Some((2, 1)));
    play(&mut term, b"\x1b[9;1H");
    assert_eq!(cursor(&term), Some((3, 0)));
    play(&mut term, b"\x1b[?6l");
    assert_eq!(cursor(&term), Some((0, 0)));
}

#[test]
fn insert_and_delete_shift_within_the_line_and_the_region() {
    let mut term = model(6, 4, 10);
    play(&mut term, b"abcdef\x1b[1;3H\x1b[2@");
    assert_eq!(screen(&term), ["ab  cd", "", "", ""]);
    play(&mut term, b"\x1b[3P");
    assert_eq!(screen(&term), ["abd", "", "", ""]);
    play(&mut term, b"\x1b[2;1Hrow2\x1b[3;1Hrow3\x1b[2;1H\x1b[L");
    assert_eq!(screen(&term), ["abd", "", "row2", "row3"]);
    play(&mut term, b"\x1b[2M");
    assert_eq!(screen(&term), ["abd", "row3", "", ""]);
    play(&mut term, b"\x1b[1;2H\x1b[4X");
    assert_eq!(screen(&term), ["a", "row3", "", ""]);
}

#[test]
fn insert_mode_pushes_the_line_right() {
    let mut term = model(5, 1, 0);
    play(&mut term, b"abc\x1b[1;1H\x1b[4hXY\x1b[4lZ");
    assert_eq!(screen(&term), ["XYZbc"]);
}

#[test]
fn the_erases_cover_their_parts_and_ed_3_clears_history_alone() {
    let mut term = model(4, 3, 10);
    play(&mut term, b"old\r\naaaa\r\nbbbb\r\ncccc\x1b[2;3H\x1b[K");
    assert_eq!(screen(&term), ["aaaa", "bb", "cccc"]);
    play(&mut term, b"\x1b[1K");
    assert_eq!(screen(&term), ["aaaa", "", "cccc"]);
    play(&mut term, b"\x1b[J");
    assert_eq!(screen(&term), ["aaaa", "", ""]);
    play(&mut term, b"\x1b[3J");
    assert!(history(&term).is_empty());
    assert_eq!(screen(&term), ["aaaa", "", ""]);
    play(&mut term, b"\x1b[2J");
    assert_eq!(screen(&term), ["", "", ""]);
}

#[test]
fn tab_stops_are_every_eighth_column_until_the_program_sets_its_own() {
    let mut term = model(20, 1, 0);
    play(&mut term, b"\tx");
    assert_eq!(cursor(&term), Some((0, 9)));
    play(&mut term, b"\x1b[3I");
    assert_eq!(cursor(&term), Some((0, 19)));
    play(&mut term, b"\x1b[2Z");
    assert_eq!(cursor(&term), Some((0, 8)));
    play(&mut term, b"\x1b[3g\x1b[1;4H\x1bH\x1b[1;1H\t");
    assert_eq!(cursor(&term), Some((0, 3)));
    play(&mut term, b"\x1b[0g\x1b[1;1H\t");
    assert_eq!(cursor(&term), Some((0, 19)));
}

#[test]
fn a_saved_cursor_is_per_screen_and_restoring_none_goes_home() {
    let mut term = model(10, 5, 0);
    play(&mut term, b"\x1b[3;4H\x1b7\x1b[1;1H\x1b8");
    assert_eq!(cursor(&term), Some((2, 3)));
    play(&mut term, b"\x1b[?47h\x1b8");
    assert_eq!(cursor(&term), Some((0, 0)));
    play(&mut term, b"\x1b[?47l\x1b[5;5H\x1b[u");
    assert_eq!(cursor(&term), Some((2, 3)));
}

#[test]
fn ris_clears_screens_history_and_modes_and_still_counts_a_change() {
    let mut term = model(4, 2, 10);
    play(&mut term, b"a\r\nb\r\nc\x1b[?25l\x1b[?1h\x1b]2;t\x07");
    let before = term.grid().generation();
    play(&mut term, b"\x1bc");
    assert_eq!(screen(&term), ["", ""]);
    assert!(history(&term).is_empty());
    assert_eq!(cursor(&term), Some((0, 0)));
    assert_eq!(term.grid().title(), None);
    assert!(term.grid().generation() > before);
}

#[test]
fn the_generation_moves_only_when_what_is_drawn_changes() {
    let mut term = model(4, 2, 10);
    let start = term.grid().generation();
    play(
        &mut term,
        b"\x1b[6n\x1b[c\x1b]2;title\x07\x1b[?1h\x1b[?2004h\x1b[1m\x07\x1b[>c\x1bP$qm\x1b\\",
    );
    assert_eq!(term.grid().generation(), start);
    assert_eq!(term.grid().title(), Some("title"));
    play(&mut term, b"x");
    assert!(term.grid().generation() > start);
}

#[test]
fn a_view_scrolled_back_sees_history_and_no_cursor_off_its_rows() {
    let mut term = model(3, 2, 10);
    play(&mut term, b"1\r\n2\r\n3\r\n4");
    let back = term.snapshot(Scroll(2));
    let rows: Vec<String> = back.cells.chunks(3).map(super::text).collect();
    assert_eq!(rows, ["1", "2"]);
    assert_eq!(back.cursor, None);
    assert_eq!(back.history, 2);
    // The cursor's row is in view one line back once it is on the top row.
    assert_eq!(term.snapshot(Scroll(1)).cursor, None);
    play(&mut term, b"\x1b[H");
    assert_eq!(term.snapshot(Scroll(1)).cursor.map(|at| at.row), Some(1));
    // Past the top is the top.
    assert_eq!(term.snapshot(Scroll(u32::MAX)).cells, back.cells);
}

#[test]
fn a_shorter_screen_drops_blank_rows_first_then_keeps_its_top_in_history() {
    let mut term = model(4, 4, 10);
    play(&mut term, b"a\r\nb");
    resize(&mut term, 4, 2);
    assert_eq!(screen(&term), ["a", "b"]);
    assert!(history(&term).is_empty());
    play(&mut term, b"\r\nc");
    resize(&mut term, 4, 1);
    assert_eq!(screen(&term), ["c"]);
    assert_eq!(history(&term), ["a", "b"]);
    assert_eq!(cursor(&term), Some((0, 1)));
    // Taller again: the lines come back from history.
    resize(&mut term, 4, 3);
    assert_eq!(screen(&term), ["a", "b", "c"]);
    assert_eq!(cursor(&term), Some((2, 1)));
}

#[test]
fn a_narrower_screen_reflows_and_a_wider_one_takes_the_lines_back() {
    let mut term = model(6, 2, 10);
    play(&mut term, b"abcdef\r\n");
    resize(&mut term, 3, 2);
    assert_eq!(history(&term), ["abc"]);
    assert_eq!(screen(&term), ["def", ""]);
    assert_eq!(cursor(&term), Some((1, 0)));
    resize(&mut term, 8, 2);
    assert!(history(&term).is_empty());
    assert_eq!(screen(&term), ["abcdef", ""]);
    // Columns gained have xterm's default stops (none below 8 here).
    play(&mut term, b"\x1b[1;1H\t!");
    assert_eq!(screen(&term), ["abcdef !", ""]);
    assert_eq!(cursor(&term), Some((0, 7)));
}

fn resize(term: &mut TerminalModel, cols: u16, rows: u16) {
    let size = GridSize::new(cols, rows);
    assert!(size.is_ok());
    if let Ok(size) = size {
        term.resize(size);
    }
}
