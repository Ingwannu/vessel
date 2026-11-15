//! UTF-8 offsets and one-field scoring are independent of rendered fonts.
use crate::matcher::Query;
use crate::palette::{DisplayPath, MarkedSubtitle, Subtitle};

#[test]
fn whole_sentence_search_and_scores_are_preserved_after_semantic_split() {
    for (status, path) in [
        ("running", "/w/api"),
        ("실행 중", "/작업/경로"),
        ("İ", "C:\\개발\\İnput"),
    ] {
        let subtitle = Subtitle::StatusPath {
            status: status.to_owned(),
            path: DisplayPath::new(path),
        };
        let legacy = format!("{status} · {path}");
        assert_eq!(subtitle.search_text(), legacy);
        for query in [
            "",
            "running api",
            "중 경로",
            "·",
            "i",
            "input",
            "no such path",
        ] {
            let query = Query::new(query);
            assert_eq!(
                query.score(&["Title", &subtitle.search_text()], None),
                query.score(&["Title", &legacy], None)
            );
        }
    }
}

#[test]
fn separator_and_korean_matches_remain_in_their_original_part_bytes() {
    let subtitle = Subtitle::StatusPath {
        status: "실행 중".to_owned(),
        path: DisplayPath::new("/작업/경로"),
    };
    let MarkedSubtitle::StatusPath { prefix, path } = subtitle.marked(&Query::new("중 경로"))
    else {
        panic!("typed path lost");
    };
    assert_eq!(prefix.text, "실행 중 · ");
    assert_eq!(prefix.marks, span(7, 10));
    assert_eq!(path.text, "/작업/경로");
    assert_eq!(path.marks, span(8, 14));
    let MarkedSubtitle::StatusPath { prefix, path } = subtitle.marked(&Query::new("·")) else {
        panic!("typed path lost");
    };
    assert_eq!(prefix.marks, span(11, 13));
    assert!(path.marks.is_empty());
}

#[test]
fn expanded_lowercase_uses_original_utf8_ranges_and_empty_query_has_no_marks() {
    let subtitle = Subtitle::StatusPath {
        status: "İ".to_owned(),
        path: DisplayPath::new("/İ/경로"),
    };
    // One mark per query token, at its best match in the whole sentence
    // (Paseo's `findHighlightRanges`): the status's İ, two bytes, and not
    // the path's.
    let MarkedSubtitle::StatusPath { prefix, path } = subtitle.marked(&Query::new("i")) else {
        panic!("typed path lost");
    };
    assert_eq!(prefix.marks, span(0, 2));
    assert!(path.marks.is_empty());
    // A token found only in the path is measured from the path's start,
    // after the expanded İ before it.
    let MarkedSubtitle::StatusPath { prefix, path } = subtitle.marked(&Query::new("경로")) else {
        panic!("typed path lost");
    };
    assert!(prefix.marks.is_empty());
    assert_eq!(path.marks, span(4, 10));
    for query in ["", "unmatched"] {
        let MarkedSubtitle::StatusPath { prefix, path } = subtitle.marked(&Query::new(query))
        else {
            panic!("typed path lost");
        };
        assert!(prefix.marks.is_empty());
        assert!(path.marks.is_empty());
    }
}

fn span(start: usize, end: usize) -> Vec<std::ops::Range<usize>> {
    std::iter::once(start..end).collect()
}
