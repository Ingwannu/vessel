//! Sanitizing presentation ranges at the kit boundary. The matcher uses
//! UTF-8 byte offsets; callers can be stale, so malformed ranges are
//! ignored rather than letting a highlight cut a Korean syllable.

use std::ops::Range;

pub(crate) fn valid(text: &str, ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut ranges: Vec<_> = ranges
        .into_iter()
        .filter(|range| range.start < range.end && text.get(range.clone()).is_some())
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

#[cfg(test)]
mod tests {
    use super::valid;

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
}
