//! Recorded byte sequences for the grid: colour, Hangul, the alternate
//! screen, bracketed paste, and who answers a query.

use crate::prefs::Preferences;

use super::{
    Cell, Colour, FEED_BUDGET, FindBound, Glyph, GridSize, Key, KeysTyped, Mods, NodePlatform,
    OtherAnswerer, Place, QueryDuty, ScrollbackLines, TerminalError, TerminalModel, TerminalOrigin,
    ViewerFocus, Weight, query_duty, quote_path,
};

fn push(model: &mut TerminalModel, bytes: &[u8]) {
    let fed = model.feed(bytes);
    assert_eq!(fed.consumed, bytes.len());
}

fn grid(cols: u16, rows: u16, scroll: u32) -> TerminalModel {
    let size = GridSize::new(cols, rows).expect("size");
    let scroll = ScrollbackLines::new(scroll).expect("scroll");
    TerminalModel::new(size, scroll)
}

fn glyph_at(model: &TerminalModel, row: u16, col: u16) -> Glyph {
    let snap = model.snapshot();
    let cols = usize::from(snap.cols);
    let index = usize::from(row)
        .saturating_mul(cols)
        .saturating_add(usize::from(col));
    snap.cells
        .get(index)
        .copied()
        .map(Cell::glyph)
        .expect("cell")
}

fn cell_at(model: &TerminalModel, row: u16, col: u16) -> Cell {
    let snap = model.snapshot();
    let cols = usize::from(snap.cols);
    let index = usize::from(row)
        .saturating_mul(cols)
        .saturating_add(usize::from(col));
    snap.cells.get(index).copied().expect("cell")
}

fn home_replies() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"\x1b[?62;4;22c");
    out.extend_from_slice(b"\x1b[?62;4;22c");
    out.extend_from_slice(b"\x1b[0n");
    out.extend_from_slice(b"\x1b[1;1R");
    out.extend_from_slice(b"\x1b[?1;1R");
    out.extend_from_slice(b"\x1b]10;rgb:e6e6/e6e6/e6e6\x1b\\");
    out.extend_from_slice(b"\x1b]11;rgb:0b0b/0b0b/0b0b\x1b\\");
    out.extend_from_slice(b"\x1b]12;rgb:e6e6/e6e6/e6e6\x1b\\");
    out
}

fn queries() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"\x1b[c");
    out.extend_from_slice(b"\x1b[0c");
    out.extend_from_slice(b"\x1b[5n");
    out.extend_from_slice(b"\x1b[6n");
    out.extend_from_slice(b"\x1b[?6n");
    out.extend_from_slice(b"\x1b]10;?\x1b\\");
    out.extend_from_slice(b"\x1b]11;?\x1b\\");
    out.extend_from_slice(b"\x1b]12;?\x1b\\");
    out
}

#[test]
fn a_coloured_run_keeps_weight_and_index() {
    let mut model = grid(8, 2, 10);
    let fed = model.feed(b"\x1b[1;31mX");
    assert!(fed.replies.is_empty());
    let cell = cell_at(&model, 0, 0);
    assert_eq!(cell.glyph(), Glyph::Scalar('X'));
    assert_eq!(cell.attrs().weight, Weight::Bold);
    assert_eq!(cell.attrs().fg, Colour::Indexed(1));
}

#[test]
fn hangul_is_a_wide_scalar_and_a_continuation() {
    let mut model = grid(4, 1, 10);
    push(&mut model, "한".as_bytes());
    assert_eq!(glyph_at(&model, 0, 0), Glyph::Scalar('한'));
    assert_eq!(glyph_at(&model, 0, 1), Glyph::Continuation);
}

#[test]
fn a_hangul_syllable_split_across_two_feeds_is_one_scalar() {
    let mut model = grid(4, 1, 10);
    let first = model.feed(&[0xED]);
    assert_eq!(first.consumed, 1);
    assert_eq!(glyph_at(&model, 0, 0), Glyph::Empty);
    push(&mut model, &[0x95, 0x9C]);
    assert_eq!(glyph_at(&model, 0, 0), Glyph::Scalar('한'));
    assert_eq!(glyph_at(&model, 0, 1), Glyph::Continuation);
}

#[test]
fn the_alternate_screen_does_not_keep_its_text() {
    let mut model = grid(4, 2, 10);
    push(&mut model, b"A\x1b[?1049hB\x1b[?1049l");
    assert_eq!(glyph_at(&model, 0, 0), Glyph::Scalar('A'));
}

#[test]
fn bracketed_paste_follows_the_mode_the_program_set() {
    let mut model = grid(8, 2, 10);
    assert_eq!(model.paste("hi"), b"hi");
    push(&mut model, b"\x1b[?2004h");
    assert_eq!(model.paste("hi"), b"\x1b[200~hi\x1b[201~");
    push(&mut model, b"\x1b[?2004l");
    assert_eq!(model.paste("hi"), b"hi");
}

#[test]
fn arrows_follow_normal_and_application_cursor_mode() {
    let mut model = grid(8, 2, 10);
    assert_eq!(model.key(Key::Up, Mods::Plain), b"\x1b[A");
    push(&mut model, b"\x1b[?1h");
    assert_eq!(model.key(Key::Up, Mods::Plain), b"\x1bOA");
    push(&mut model, b"\x1b[?1l");
    assert_eq!(model.key(Key::Left, Mods::Plain), b"\x1b[D");
}

#[test]
fn a_live_cursor_query_names_the_cells_row_and_column() {
    let mut model = grid(8, 4, 10);
    push(&mut model, b"\x1b[2;3H");
    let fed = model.feed(b"\x1b[6n");
    assert_eq!(fed.replies, b"\x1b[2;3R");
}

#[test]
fn a_replayed_cursor_query_types_nothing() {
    let mut model = grid(8, 2, 10);
    let fed = model.replay(b"\x1b[6n");
    assert!(fed.replies.is_empty());
    assert_eq!(model.snapshot().cursor.row, 0);
    assert_eq!(model.snapshot().cursor.col, 0);
}

#[test]
fn the_opener_answers_before_anyone_qualifies() {
    let duty = query_duty(
        TerminalOrigin::Opened,
        ViewerFocus::Lost,
        KeysTyped::Never,
        OtherAnswerer::None,
    );
    assert_eq!(duty, QueryDuty::Answer);
    let mut model = grid(8, 2, 10);
    let fed = model.feed(&queries());
    assert_eq!(fed.replies, home_replies());
}

#[test]
fn nobody_answers_a_live_query_when_no_viewer_may() {
    let duty = query_duty(
        TerminalOrigin::Attached,
        ViewerFocus::Lost,
        KeysTyped::Never,
        OtherAnswerer::None,
    );
    assert_eq!(duty, QueryDuty::Silent);
    let focused = query_duty(
        TerminalOrigin::Opened,
        ViewerFocus::Held,
        KeysTyped::Once,
        OtherAnswerer::Qualifies,
    );
    assert_eq!(focused, QueryDuty::Silent);
    let mut model = grid(8, 2, 10);
    let fed = model.replay(&queries());
    assert!(fed.replies.is_empty());
}

#[test]
fn a_qualified_viewer_answers_and_a_second_window_does_not() {
    assert_eq!(
        query_duty(
            TerminalOrigin::Attached,
            ViewerFocus::Held,
            KeysTyped::Once,
            OtherAnswerer::None,
        ),
        QueryDuty::Answer
    );
    assert_eq!(
        query_duty(
            TerminalOrigin::Opened,
            ViewerFocus::Lost,
            KeysTyped::Never,
            OtherAnswerer::Qualifies,
        ),
        QueryDuty::Silent
    );
}

#[test]
fn an_unknown_sequence_does_not_drop_the_text_after_it() {
    let mut model = grid(8, 2, 10);
    push(&mut model, b"\x1b[999~Z");
    assert_eq!(glyph_at(&model, 0, 0), Glyph::Scalar('Z'));
}

#[test]
fn one_feed_stops_at_the_pty_read_size() {
    let mut model = grid(8, 2, 10);
    let bytes = vec![b'A'; FEED_BUDGET.saturating_add(3)];
    let first = model.feed(&bytes);
    assert_eq!(first.consumed, FEED_BUDGET);
    let rest = bytes.get(first.consumed..).unwrap_or(&[]);
    let second = model.feed(rest);
    assert_eq!(second.consumed, 3);
}

#[test]
fn scrollback_drops_the_oldest_line() {
    let mut model = grid(4, 2, 1);
    push(&mut model, b"a\r\nb\r\nc\r\nd\r\n");
    let kept = model.select(Place { line: 0, col: 0 }, Place { line: 2, col: 0 });
    assert!(kept.is_ok());
    let past = model.select(Place { line: 3, col: 0 }, Place { line: 3, col: 0 });
    assert_eq!(past, Err(TerminalError::Place));
}

#[test]
fn a_soft_wrapped_line_reflows_when_the_grid_narrows() {
    let mut model = grid(4, 2, 10);
    push(&mut model, b"abcdX");
    model.resize(GridSize::new(2, 2).expect("size"));
    assert_eq!(glyph_at(&model, 0, 0), Glyph::Scalar('c'));
    assert_eq!(glyph_at(&model, 0, 1), Glyph::Scalar('d'));
    assert_eq!(glyph_at(&model, 1, 0), Glyph::Scalar('X'));
}

#[test]
fn find_is_case_insensitive_and_stops_at_the_ends() {
    let mut model = grid(8, 2, 10);
    push(&mut model, b"AbC");
    let found = model.find("abc");
    assert_eq!(found.count(), 1);
    assert_eq!(found.bound(), FindBound::All);
    assert_eq!(found.current(), Some(Place { line: 0, col: 0 }));
    let next = model.find_next();
    assert_eq!(next.current(), Some(Place { line: 0, col: 0 }));
    let previous = model.find_previous();
    assert_eq!(previous.current(), Some(Place { line: 0, col: 0 }));
    let none = model.find("z");
    assert_eq!(none.count(), 0);
    assert_eq!(none.current(), None);
}

#[test]
fn find_skips_the_continuation_of_a_wide_scalar() {
    let mut model = grid(4, 1, 10);
    push(&mut model, "한x".as_bytes());
    let found = model.find("x");
    assert_eq!(found.current(), Some(Place { line: 0, col: 2 }));
}

#[test]
fn a_query_does_not_bump_the_generation_and_text_does() {
    let mut model = grid(8, 2, 10);
    let before = model.snapshot().generation;
    push(&mut model, b"A");
    let after = model.snapshot().generation;
    assert_ne!(before, after);
    push(&mut model, b"\x1b[6n");
    assert_eq!(model.snapshot().generation, after);
}

#[test]
fn a_posix_path_is_quoted_and_a_dangerous_one_is_refused() {
    assert_eq!(quote_path("a b", NodePlatform::Posix).unwrap(), "'a b'");
    assert_eq!(quote_path("a\\b", NodePlatform::Posix).unwrap(), "'a\\\\b'");
    assert_eq!(quote_path("a'b", NodePlatform::Posix).unwrap(), "'a\\'b'");
    assert_eq!(
        quote_path("a'b\"c", NodePlatform::Posix).unwrap(),
        "$'a\\'b\"c'"
    );
    let refused = quote_path("a;b", NodePlatform::Posix).unwrap_err();
    assert_eq!(refused, TerminalError::Path);
    assert!(!refused.to_string().contains("a;b"));
    assert_eq!(quote_path("a b", NodePlatform::Windows).unwrap(), "\"a b\"");
    assert_eq!(quote_path("a;b", NodePlatform::Windows).unwrap(), "a;b");
}

#[test]
fn scrollback_on_preferences_is_absent_at_the_default() {
    let prefs = Preferences::default();
    assert_eq!(prefs.terminal_scrollback, ScrollbackLines::DEFAULT);
    let text = serde_json::to_string(&prefs).unwrap();
    assert!(!text.contains("terminal_scrollback"));
    let read: Preferences = serde_json::from_str(r#"{"terminal_scrollback":5000}"#).unwrap();
    assert_eq!(read.terminal_scrollback.get(), 5_000);
    let bad: Preferences = serde_json::from_str(r#"{"terminal_scrollback":1000001}"#).unwrap();
    assert_eq!(bad.terminal_scrollback, ScrollbackLines::DEFAULT);
    assert_eq!(
        serde_json::from_str::<Preferences>("{}").unwrap(),
        Preferences::default()
    );
}
