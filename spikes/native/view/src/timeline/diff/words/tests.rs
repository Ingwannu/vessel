use super::{
    MAX_LINES, MAX_WORD_BYTES, MAX_WORD_CELLS, MAX_WORD_TOKENS, changed_pair, word_highlights,
    words,
};
use crate::timeline::{DiffLine, Side, diff_lines};
use std::ops::Range;

fn changed(old: &str, new: &str) -> (Vec<String>, Vec<String>) {
    let mut budget = MAX_WORD_CELLS;
    let (old_ranges, new_ranges) = changed_pair(old, new, &mut budget).unwrap();
    let text = |source: &str, ranges: Vec<Range<usize>>| {
        ranges
            .into_iter()
            .map(|range| source.get(range).unwrap().to_owned())
            .collect()
    };
    (text(old, old_ranges), text(new, new_ranges))
}

#[test]
fn the_public_annotation_boundary_caps_rows_even_for_an_oversized_caller() {
    let lines = vec![
        DiffLine {
            side: Side::Removed,
            number: None,
            text: "kept"
        };
        MAX_LINES.saturating_add(1)
    ];
    let marks = word_highlights(&lines);
    assert_eq!(marks.0.len(), MAX_LINES);
    assert!(marks.for_line(MAX_LINES).is_empty());
    assert_eq!(lines.last().unwrap().text, "kept");
}

#[test]
fn equal_words_stay_plain_while_only_the_changed_word_is_emphasized() {
    assert_eq!(
        changed("const value = old;", "const value = new;"),
        (vec!["old".to_owned()], vec!["new".to_owned()])
    );
    assert_eq!(changed("same", "same"), (vec![], vec![]));
    assert_eq!(
        changed("a b", "b a"),
        (vec!["a ".to_owned()], vec![" a".to_owned()])
    );
}

#[test]
fn punctuation_and_unicode_keep_the_reference_word_runs_and_valid_byte_boundaries() {
    assert_eq!(
        words("a_2 + 한글🙂 = b").unwrap(),
        ["a_2", " + 한글🙂 = ", "b"]
    );
    assert_eq!(
        changed("한글🙂 = old", "한글🙂 = new"),
        (vec!["old".to_owned()], vec!["new".to_owned()])
    );
    assert_eq!(
        changed("x += 2", "x -= 2"),
        (vec![" += ".to_owned()], vec![" -= ".to_owned()])
    );
}

#[test]
fn only_the_adjacent_removed_added_pair_receives_word_ranges() {
    let (lines, _) = diff_lines(Some("first old\nsecond old"), "first new\nsecond new");
    let marks = word_highlights(&lines);
    assert!(marks.for_line(0).is_empty());
    assert!(!marks.for_line(1).is_empty());
    assert!(!marks.for_line(2).is_empty());
    assert!(marks.for_line(3).is_empty());
    assert!(marks.for_line(usize::MAX).is_empty());
    for (i, line) in lines.iter().enumerate() {
        assert!(
            marks
                .for_line(i)
                .iter()
                .all(|range| line.text.get(range.clone()).is_some())
        );
    }
}

#[test]
fn insertions_deletions_and_work_limits_retain_whole_line_content() {
    assert_eq!(changed("", "new"), (vec![], vec!["new".to_owned()]));
    assert_eq!(changed("old", ""), (vec!["old".to_owned()], vec![]));
    assert!(changed_pair("a", "b", &mut 0).is_none());
    let mut budget = 4;
    assert!(changed_pair("a", "b", &mut budget).is_some());
    assert_eq!(budget, 0);
    assert!(changed_pair("c", "d", &mut budget).is_none());
    let many = "a ".repeat(MAX_WORD_TOKENS);
    assert!(words(&many).is_none());
    let long = "가".repeat(MAX_WORD_BYTES);
    assert!(words(&long).is_none());
}
