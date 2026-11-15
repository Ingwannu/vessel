//! Wide scalars and their continuations, and marks composed into the cell
//! before the cursor (ADR 0073's named fixtures among them).

use super::{check, cursor, model, play, screen};
use crate::terminal::TerminalModel;
use crate::terminal::grid::Glyph;

/// Row `row`'s glyphs.
fn glyphs(term: &TerminalModel, row: usize) -> Vec<Glyph> {
    term.grid()
        .screen_lines()
        .get(row)
        .map(|line| line.cells().iter().map(|cell| cell.glyph()).collect())
        .unwrap_or_default()
}

use Glyph::{Continuation as Cont, Empty, Scalar};

#[test]
fn a_hangul_syllable_is_two_cells_a_scalar_and_its_continuation() {
    let mut term = model(4, 1, 0);
    play(&mut term, "한".as_bytes());
    assert_eq!(glyphs(&term, 0), [Scalar('한'), Cont, Empty, Empty]);
    assert_eq!(cursor(&term), Some((0, 2)));
}

#[test]
fn the_same_syllable_split_across_two_feeds_is_one_wide_cell() {
    let mut term = model(4, 1, 0);
    let bytes = "한".as_bytes();
    let (head, tail) = bytes.split_at(2);
    assert_eq!(term.replay(head).consumed, 2);
    assert_eq!(term.replay(tail).consumed, 1);
    assert_eq!(glyphs(&term, 0), [Scalar('한'), Cont, Empty, Empty]);
}

#[test]
fn a_wide_scalar_that_does_not_fit_wraps_whole() {
    let mut term = model(5, 2, 0);
    play(&mut term, "abcd한".as_bytes());
    assert_eq!(screen(&term), ["abcd", "한"]);
    assert_eq!(cursor(&term), Some((1, 2)));
    // Ending on the last column, it leaves a wrap pending.
    let mut term = model(4, 2, 0);
    play(&mut term, "ab한".as_bytes());
    assert_eq!(cursor(&term), Some((0, 3)));
    play(&mut term, b"c");
    assert_eq!(screen(&term), ["ab한", "c"]);
}

#[test]
fn without_room_a_wide_scalar_is_dropped() {
    let mut term = model(4, 1, 0);
    play(&mut term, "\x1b[?7labc한d".as_bytes());
    assert_eq!(screen(&term), ["abcd"]);
    let mut term = model(1, 2, 0);
    play(&mut term, "한a".as_bytes());
    assert_eq!(screen(&term), ["a", ""]);
    check(term.grid());
}

#[test]
fn overwriting_half_of_a_wide_scalar_blanks_the_other_half() {
    let mut term = model(4, 1, 0);
    play(&mut term, "한\x1b[1;2Hx".as_bytes());
    assert_eq!(glyphs(&term, 0), [Empty, Scalar('x'), Empty, Empty]);
    play(&mut term, "\x1b[1;2H한\x1b[1;2Hy".as_bytes());
    assert_eq!(glyphs(&term, 0), [Empty, Scalar('y'), Empty, Empty]);
    play(&mut term, "\x1b[1;1H한\x1b[1;1Hz".as_bytes());
    assert_eq!(glyphs(&term, 0), [Scalar('z'), Empty, Empty, Empty]);
}

#[test]
fn erase_insert_and_delete_never_leave_half_a_pair() {
    let mut term = model(5, 1, 0);
    play(&mut term, "a한b\x1b[1;3H\x1b[X".as_bytes());
    assert_eq!(
        glyphs(&term, 0),
        [Scalar('a'), Empty, Empty, Scalar('b'), Empty]
    );
    play(&mut term, "\x1b[1;2H한\x1b[1;3H\x1b[@".as_bytes());
    assert_eq!(
        glyphs(&term, 0),
        [Scalar('a'), Empty, Empty, Empty, Scalar('b')]
    );
    play(&mut term, "\x1b[1;2H한\x1b[1;2H\x1b[P".as_bytes());
    assert_eq!(
        glyphs(&term, 0),
        [Scalar('a'), Empty, Empty, Scalar('b'), Empty]
    );
    // An insert that pushes a wide scalar half off the edge.
    play(&mut term, "\x1b[1;1Habc한\x1b[1;1H\x1b[@".as_bytes());
    assert_eq!(
        glyphs(&term, 0),
        [Empty, Scalar('a'), Scalar('b'), Scalar('c'), Empty]
    );
}

#[test]
fn a_narrower_alternate_screen_cuts_a_wide_scalar_at_the_edge_to_a_blank() {
    // The alternate screen is cut, not reflowed.
    let mut term = model(4, 1, 0);
    play(&mut term, "\x1b[?1049hab한".as_bytes());
    if let Ok(size) = crate::terminal::GridSize::new(3, 1) {
        term.resize(size);
    }
    assert_eq!(glyphs(&term, 0), [Scalar('a'), Scalar('b'), Empty]);
    check(term.grid());
}

#[test]
fn decomposed_hangul_and_accents_compose_into_one_scalar() {
    let mut term = model(6, 1, 0);
    play(&mut term, "\u{1112}\u{1161}\u{11ab}e\u{301}".as_bytes());
    assert_eq!(
        glyphs(&term, 0),
        [Scalar('한'), Cont, Scalar('é'), Empty, Empty, Empty]
    );
    assert_eq!(cursor(&term), Some((0, 3)));
    // Onto the last column while a wrap is pending.
    let mut term = model(3, 1, 0);
    play(&mut term, "abe\u{301}".as_bytes());
    assert_eq!(screen(&term), ["abé"]);
}

#[test]
fn a_mark_with_nothing_to_compose_with_is_dropped() {
    let mut term = model(4, 1, 0);
    play(&mut term, "\u{301}a\u{e34}\u{200d}".as_bytes());
    assert_eq!(screen(&term), ["a"]);
    assert_eq!(cursor(&term), Some((0, 1)));
}
