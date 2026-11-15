//! How many cells a scalar takes, and the rule that keeps a wide scalar
//! paired with its continuation.
//!
//! [Decision Log]
//! - Purpose: a Hangul syllable, a CJK ideograph or an emoji takes two
//!   cells; a combining mark takes none. The grid must place each so the
//!   cursor lands where the program expects.
//! - Choice: `unicode-width` 0.2.3's scalar width (ADR 0073), not
//!   grapheme clusters (a stated difference). A wide scalar is a `Scalar`
//!   followed by a `Continuation`; every edit that can split a pair
//!   (overwrite, erase, insert, delete, a narrower grid) blanks the half
//!   left behind, as xterm.js's `replaceCells` does at a range's edges.
//! - Combining marks: a zero-width scalar is composed into the cell
//!   before the cursor when Unicode has a composed form of the same width
//!   (`unicode-normalization`'s `compose`): decomposed Hangul (`ls` on a
//!   macOS node prints file names that way) and decomposed Latin accents
//!   become one scalar. A mark with no composed form is dropped, a stated
//!   difference (xterm.js keeps it; keeping marks would add bytes to
//!   every cell history holds, or a side table every edit must shift).

use unicode_width::UnicodeWidthChar;

use super::cell::{Cell, Glyph};

/// A scalar's width in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Width {
    /// A combining mark, a joiner or a control: no cell of its own.
    Zero,
    /// One cell.
    One,
    /// Two cells: the scalar and its continuation.
    Two,
}

/// `c`'s width (`unicode-width`, not the CJK-ambiguous variant).
pub(super) fn width(c: char) -> Width {
    match UnicodeWidthChar::width(c) {
        None | Some(0) => Width::Zero,
        Some(1) => Width::One,
        Some(_) => Width::Two,
    }
}

/// Whether `cell` holds the first half of a wide scalar.
fn wide(cell: Option<&Cell>) -> bool {
    matches!(cell.map(|cell| cell.glyph()), Some(Glyph::Scalar(c)) if width(c) == Width::Two)
}

/// Blank every half pair in `cells[from..to]` and the cell on each side:
/// a continuation not after a wide scalar, and a wide scalar not followed
/// by its continuation.
pub(super) fn fix_pairs(cells: &mut [Cell], from: usize, to: usize, blank: Cell) {
    let start = from.saturating_sub(1);
    let end = to.saturating_add(1).min(cells.len());
    for at in start..end {
        let orphan = match cells.get(at).map(|cell| cell.glyph()) {
            Some(Glyph::Continuation) => {
                !wide(at.checked_sub(1).and_then(|before| cells.get(before)))
            }
            Some(Glyph::Scalar(c)) if width(c) == Width::Two => !matches!(
                cells.get(at.saturating_add(1)).map(|cell| cell.glyph()),
                Some(Glyph::Continuation)
            ),
            Some(Glyph::Scalar(_) | Glyph::Empty) | None => false,
        };
        if orphan && let Some(cell) = cells.get_mut(at) {
            *cell = blank;
        }
    }
}

/// `base` with the mark `mark` composed into it, when Unicode has such a
/// scalar and it is as wide as `base`.
pub(super) fn compose(base: char, mark: char) -> Option<char> {
    unicode_normalization::char::compose(base, mark)
        .filter(|composed| width(*composed) == width(base))
}
