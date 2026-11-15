//! Which characters of a search result are marked, as byte ranges of the
//! text shown: the characters that made a file's name match, or the words
//! found on a line (Paseo's `components/ui/highlighted-text.tsx`, which
//! shows why a result is in the list; the marks' colour is the panel's).
//!
//! The node counts characters in Unicode scalar values over the whole
//! path, or from the start of the line, while the panel shows a name and
//! its folder apart, and a line with its indent taken off; so a position
//! is counted from where the shown text starts in the whole.

use std::collections::BTreeSet;
use std::ops::Range;

/// Byte ranges of `text` for the characters at `chars`, where `text` is
/// the part of a longer text that starts at its character `from` (from 0).
/// Characters outside `text` are left out; neighbours are joined.
#[must_use]
pub fn marks_at(text: &str, from: u32, chars: impl IntoIterator<Item = u32>) -> Vec<Range<usize>> {
    let wanted: BTreeSet<u32> = chars
        .into_iter()
        .filter_map(|at| at.checked_sub(from))
        .collect();
    let mut out: Vec<Range<usize>> = Vec::new();
    for (index, (at, character)) in text.char_indices().enumerate() {
        let Ok(index) = u32::try_from(index) else {
            break;
        };
        if !wanted.contains(&index) {
            continue;
        }
        let end = at.saturating_add(character.len_utf8());
        match out.last_mut() {
            Some(last) if last.end == at => last.end = end,
            Some(_) | None => out.push(at..end),
        }
    }
    out
}

/// The `count` characters from character `first` (from 0) of `text`, as
/// [`marks_at`] gives them.
#[must_use]
pub fn span_marks(text: &str, first: u32, count: u32) -> Vec<Range<usize>> {
    marks_at(text, 0, first..first.saturating_add(count))
}

/// How many characters of `text`'s start are white space (what a shown
/// line loses when its indent is taken off).
#[must_use]
pub fn indent_of(text: &str) -> u32 {
    let count = text
        .chars()
        .take_while(|character| character.is_whitespace())
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{indent_of, marks_at, span_marks};

    #[test]
    fn a_name_s_marks_are_counted_from_where_it_starts_in_the_path() {
        // "src/로그인.rs": the name starts at character 4.
        let name = "로그인.rs";
        let marks = marks_at(name, 4, [0, 4, 5]);
        assert_eq!(marks, vec![0..6], "character 0 of the path is the folder's");
        let marks = marks_at(name, 4, [4, 5, 6, 8]);
        assert_eq!(marks, vec![0..9, 10..11]);
        let folder = "src";
        assert_eq!(marks_at(folder, 0, [0, 1, 3, 9]), vec![0..2]);
    }

    #[test]
    fn a_line_s_mark_is_moved_by_the_indent_taken_off() {
        let line = "    export const item = 1;";
        let shown = line.trim();
        let lead = indent_of(line);
        assert_eq!(lead, 4);
        // The node's column 5 (from 1) is the shown line's character 0.
        let marks = span_marks(shown, 5_u32.saturating_sub(1).saturating_sub(lead), 6);
        assert_eq!(marks, vec![0..6]);
        assert_eq!(shown.get(0..6), Some("export"));
        assert!(
            span_marks(shown, 40, 3).is_empty(),
            "past the end is nothing"
        );
    }
}
