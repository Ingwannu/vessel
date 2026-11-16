//! Sanitizing presentation ranges at the kit boundary. The matcher uses
//! UTF-8 byte offsets; callers can be stale, so malformed ranges are
//! ignored rather than letting a highlight cut a Korean syllable.

use std::ops::Range;

pub(crate) fn valid(text: &str, ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut ranges: Vec<_> = ranges
        .into_iter()
        .filter(|range| accepts(text, range))
        .collect();
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        if let Some(last) = merged.last_mut()
            && range.start <= last.end
        {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

fn accepts(text: &str, range: &Range<usize>) -> bool {
    range.start < range.end && text.get(range.clone()).is_some()
}

/// Typed decorations cannot overlap: later conflicting runs are ignored.
/// The same UTF-8 boundary rule applies to ink and background decorations.
pub(crate) fn typed<T>(text: &str, mut runs: Vec<(Range<usize>, T)>) -> Vec<(Range<usize>, T)> {
    runs.sort_by_key(|(range, _)| (range.start, range.end));
    let mut end = 0;
    runs.into_iter()
        .filter(|(range, _)| {
            if range.start < end || !accepts(text, range) {
                return false;
            }
            end = range.end;
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{typed, valid};

    #[test]
    fn stale_or_partial_character_ranges_are_ignored() {
        assert_eq!(valid("보드", vec![1..3, 0..3, 0..9, 4..4]), vec![0..3]);
    }

    #[test]
    fn overlapping_terms_merge_but_scattered_matches_stay_separate() {
        assert_eq!(
            valid("configuration", vec![5..6, 2..4, 0..3]),
            vec![0..4, 5..6]
        );
    }
    #[test]
    fn typed_runs_reject_partial_utf8_stale_and_overlapping_ranges() {
        assert_eq!(
            typed(
                "보드",
                vec![(1..3, 1), (0..3, 2), (0..6, 3), (3..6, 4), (6..9, 5)]
            ),
            vec![(0..3, 2), (3..6, 4)]
        );
    }
}
