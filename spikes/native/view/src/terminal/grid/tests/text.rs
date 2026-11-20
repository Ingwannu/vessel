//! The grid as text: addresses that keep their place as output scrolls,
//! xterm.js's copy and double and triple clicks, Paseo's find rules, and a
//! scrollback bound changed while the terminal is open.

use super::{Scroll, history, model, play};
use crate::terminal::{
    FIND_LIMIT, Finder, ScrollbackLines, SelectBy, Selection, TerminalModel, first_from, next_of,
    previous_of, span_on,
};

fn found_texts(model: &TerminalModel, query: &str) -> Vec<String> {
    let grid = model.grid();
    let Some(mut finder) = Finder::new(query, grid) else {
        return Vec::new();
    };
    while !finder.step(grid, 3) {}
    finder
        .matches()
        .iter()
        .map(|found| grid.text_between(found.start, found.end))
        .collect()
}

#[test]
fn an_address_keeps_its_text_as_output_scrolls_it_into_history() {
    let mut terminal = model(10, 3, 100);
    play(&mut terminal, b"one\r\ntwo\r\nthree");
    let at = terminal.grid().text_at(Scroll(0), 0, 0);
    assert_eq!(terminal.grid().text_between(at, at.at_col(2)), "one");
    play(&mut terminal, b"\r\nfour\r\nfive");
    // "one" is in history now: two lines above the screen's top.
    assert_eq!(terminal.grid().text_between(at, at.at_col(2)), "one");
    assert_eq!(terminal.grid().view_row(at, Scroll(0)), None);
    assert_eq!(terminal.grid().view_row(at, Scroll(2)), Some(0));
    assert_eq!(terminal.grid().scroll_to_show(at), Scroll(2));
}

#[test]
fn a_dropped_row_has_no_text() {
    let mut terminal = model(10, 2, 1);
    play(&mut terminal, b"gone\r\nkept");
    let at = terminal.grid().text_at(Scroll(0), 0, 0);
    play(&mut terminal, b"\r\nx\r\ny\r\nz");
    assert_eq!(history(&terminal).len(), 1);
    assert_eq!(terminal.grid().text_between(at, at.at_col(3)), "");
}

#[test]
fn a_copy_joins_soft_wraps_and_leaves_out_trailing_blanks() {
    let mut terminal = model(5, 4, 100);
    // "abcdefg" wraps after five cells; "hi   " ends hard with blanks.
    play(&mut terminal, b"abcdefg\r\nhi   \r\nlast");
    let grid = terminal.grid();
    let start = grid.text_at(Scroll(0), 0, 0);
    let end = grid.text_at(Scroll(0), 3, 1);
    assert_eq!(grid.text_between(start, end), "abcdefg\nhi\nla");
    // Backwards is the same text.
    assert_eq!(grid.text_between(end, start), "abcdefg\nhi\nla");
}

#[test]
fn double_and_triple_clicks_select_a_word_and_a_line_across_a_wrap() {
    let mut terminal = model(8, 3, 100);
    play(&mut terminal, b"cd (src/main.rs) ok");
    let grid = terminal.grid();
    // "src/main.rs" spans the wrap after eight cells.
    let inside = grid.text_at(Scroll(0), 1, 0);
    let (start, end) = grid.word_around(inside);
    assert_eq!(grid.text_between(start, end), "src/main.rs");
    let mut words = Selection::new(inside, SelectBy::Words);
    assert_eq!(words.text(grid), "src/main.rs");
    words.extend(grid.text_at(Scroll(0), 2, 2));
    assert_eq!(words.text(grid), "src/main.rs) ok");
    let line = Selection::new(inside, SelectBy::Lines);
    assert_eq!(line.text(grid), "cd (src/main.rs) ok");
    // A separator alone is its own word.
    let paren = grid.text_at(Scroll(0), 0, 3);
    let (start, end) = grid.word_around(paren);
    assert_eq!(grid.text_between(start, end), "(");
}

#[test]
fn a_selection_never_splits_a_wide_scalar() {
    let mut terminal = model(10, 2, 100);
    play(&mut terminal, "가나다".as_bytes());
    let grid = terminal.grid();
    // From the right half of 가 (col 1) to the right half of 나 (col 3).
    let mut cells = Selection::new(grid.text_at(Scroll(0), 0, 1), SelectBy::Cells);
    assert!(cells.is_empty(), "a press alone holds nothing");
    cells.extend(grid.text_at(Scroll(0), 0, 3));
    assert_eq!(cells.text(grid), "가나");
}

#[test]
fn a_span_covers_the_columns_of_each_row_between_the_ends() {
    let mut terminal = model(6, 3, 100);
    play(&mut terminal, b"aaaaaa\r\nbbbbbb\r\ncccccc");
    let grid = terminal.grid();
    let range = (grid.text_at(Scroll(0), 0, 4), grid.text_at(Scroll(0), 2, 1));
    let row = |view_row| span_on(range, grid.text_at(Scroll(0), view_row, 0), 6);
    assert_eq!(row(0), Some((4, 5)));
    assert_eq!(row(1), Some((0, 5)));
    assert_eq!(row(2), Some((0, 1)));
}

#[test]
fn find_is_literal_case_insensitive_and_crosses_a_soft_wrap() {
    let mut terminal = model(6, 4, 100);
    play(&mut terminal, b"Error: ERROR\r\nan error.*\r\n(e");
    assert_eq!(found_texts(&terminal, "error"), ["Error", "ERROR", "error"]);
    // "Error: ERROR" wraps after six cells, inside the second word.
    assert_eq!(found_texts(&terminal, "r: er"), ["r: ER"]);
    // Regex characters are literal text.
    assert_eq!(found_texts(&terminal, ".*"), [".*"]);
    assert_eq!(found_texts(&terminal, "(e"), ["(e"]);
    assert!(Finder::new("", terminal.grid()).is_none());
}

#[test]
fn matches_do_not_overlap_and_trailing_blanks_never_match() {
    let mut terminal = model(10, 2, 100);
    play(&mut terminal, b"aaaa  ");
    assert_eq!(found_texts(&terminal, "aa"), ["aa", "aa"]);
    assert!(found_texts(&terminal, "a   ").is_empty());
}

#[test]
fn the_count_stops_at_the_limit() {
    let mut terminal = model(20, 2, 3_000);
    let line = b"x x x x x x x x x x\r\n";
    for _ in 0..2_001 {
        play(&mut terminal, line);
    }
    let grid = terminal.grid();
    let Some(mut finder) = Finder::new("x", grid) else {
        panic!("a query");
    };
    while !finder.step(grid, 500) {}
    assert_eq!(finder.matches().len(), FIND_LIMIT);
    assert!(finder.limited());
}

#[test]
fn the_first_match_shown_is_at_or_below_the_view_and_moves_round() {
    let mut terminal = model(10, 2, 100);
    play(&mut terminal, b"hit\r\nmiss\r\nhit\r\nhit");
    let grid = terminal.grid();
    let Some(mut finder) = Finder::new("hit", grid) else {
        panic!("a query");
    };
    while !finder.step(grid, 10) {}
    let matches = finder.matches();
    assert_eq!(matches.len(), 3);
    // The view shows the last two rows: the first match there is the second.
    let top = grid.text_at(Scroll(0), 0, 0);
    assert_eq!(first_from(matches, top), Some(1));
    assert_eq!(next_of(2, 3), 0);
    assert_eq!(previous_of(0, 3), 2);
    assert_eq!(first_from(&[], top), None);
}

#[test]
fn a_lower_scrollback_drops_the_oldest_lines_at_once() {
    let mut terminal = model(10, 2, 100);
    play(&mut terminal, b"1\r\n2\r\n3\r\n4\r\n5\r\n6");
    assert_eq!(history(&terminal), ["1", "2", "3", "4"]);
    let before = terminal.grid().generation();
    terminal.set_scrollback(ScrollbackLines::clamped(2));
    assert_eq!(history(&terminal), ["3", "4"]);
    assert!(terminal.grid().generation() > before);
    terminal.set_scrollback(ScrollbackLines::clamped(10));
    play(&mut terminal, b"\r\n7\r\n8");
    assert_eq!(history(&terminal), ["3", "4", "5", "6"]);
}
