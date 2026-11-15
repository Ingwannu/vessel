//! Edit previews preserve source lines and never invent hunk positions.

use super::*;

#[test]
fn a_local_edit_keeps_unchanged_context_and_blank_lines() {
    let (rows, cut) = diff_lines(Some("first\r\n\r\nold\r\nlast"), "first\n\nnew\nlast");
    assert!(!cut);
    assert_eq!(
        rows.iter()
            .map(|row| (row.side, row.text))
            .collect::<Vec<_>>(),
        [
            (Side::Context, "first"),
            (Side::Context, ""),
            (Side::Removed, "old"),
            (Side::Added, "new"),
            (Side::Context, "last"),
        ]
    );
    assert_eq!(
        rows.iter().map(|row| row.number).collect::<Vec<_>>(),
        [Some(1), Some(2), Some(3), Some(3), Some(4)]
    );
    assert!(diff_lines(None, "").0.is_empty());
    assert!(diff_lines(Some(""), "").0.is_empty());
}

#[test]
fn a_patch_uses_hunk_offsets_and_keeps_literal_header_like_content() {
    let (rows, cut) = unified_lines(
        "--- a/file\n+++ b/file\n@@ -8,2 +12,2 @@ section\n---literal\n+++literal\n kept\n\\ No newline at end of file",
    );
    assert!(!cut);
    assert_eq!(
        rows.iter()
            .map(|row| (row.side, row.number, row.text))
            .collect::<Vec<_>>(),
        [
            (Side::Header, None, "@@ -8,2 +12,2 @@ section"),
            (Side::Removed, Some(8), "--literal"),
            (Side::Added, Some(12), "++literal"),
            (Side::Context, Some(13), "kept"),
            (Side::Header, None, "\\ No newline at end of file"),
        ]
    );
}

#[test]
fn malformed_offsets_are_unnumbered_and_input_work_is_bounded() {
    let (rows, _) = unified_lines("@@ invalid @@\n-old\n+new");
    assert!(rows.iter().all(|row| row.number.is_none()));
    let long = "+line\n".repeat(MAX_LINES.saturating_add(1));
    let (rows, cut) = unified_lines(&long);
    assert!(cut);
    assert_eq!(rows.len(), MAX_LINES);
    let (rows, cut) = diff_lines(Some(&long), &long);
    assert!(cut);
    assert_eq!(rows.len(), MAX_LINES);
}

#[test]
fn file_headers_after_a_hunk_do_not_become_source_changes() {
    let text = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\ndiff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -20 +30 @@\n-x\n+y";
    let (rows, _) = unified_lines(text);
    assert_eq!(
        rows.iter()
            .filter(|row| row.side != Side::Header)
            .map(|row| (row.text, row.number))
            .collect::<Vec<_>>(),
        [
            ("old", Some(1)),
            ("new", Some(1)),
            ("x", Some(20)),
            ("y", Some(30))
        ]
    );
}
