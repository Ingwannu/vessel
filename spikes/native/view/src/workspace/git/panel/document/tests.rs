//! Paseo utils/diff-layout.test.ts cases through the existing numbered rows.
use super::*;
use crate::workspace::diff_rows::diff_rows;
use vessel_proto::workspace::{DiffLine, Hunk};

fn ticket(path: &str) -> FileTicket {
    let mut reads = super::super::PanelReads::new(
        super::super::RepositoryKey {
            node: vessel_proto::NodeId::random(),
            root: vessel_proto::workspace::WorkspaceRoot::Project {
                project: vessel_proto::ProjectId::random(),
            },
        },
        super::super::Comparison::default(),
    );
    let mut keys = crate::workspace::git::RequestKeys::default();
    reads.begin_file(path.parse().unwrap(), keys.issue().unwrap())
}

fn hunk(old_start: u32, new_start: u32, lines: &[(LineKind, &str)]) -> Hunk {
    Hunk {
        old_start,
        new_start,
        old_lines: u32::try_from(
            lines
                .iter()
                .filter(|(kind, _)| matches!(kind, LineKind::Context | LineKind::Removed))
                .count(),
        )
        .unwrap(),
        new_lines: u32::try_from(
            lines
                .iter()
                .filter(|(kind, _)| matches!(kind, LineKind::Context | LineKind::Added))
                .count(),
        )
        .unwrap(),
        section: None,
        lines: lines
            .iter()
            .map(|(kind, text)| DiffLine {
                kind: *kind,
                text: (*text).into(),
                cut: false,
            })
            .collect(),
    }
}

fn model(lines: &[(LineKind, &str)], layout: Layout) -> DiffDocument {
    DiffDocument::new(
        &ticket("example.ts"),
        diff_rows(&[hunk(10, 20, lines)], 0).into(),
        layout,
    )
}

fn pairs(model: &DiffDocument) -> Vec<(Option<&str>, Option<&str>)> {
    let text = |cell: Option<DiffCell>| {
        cell.map(|cell| match model.source(cell.source()).unwrap() {
            DiffRow::Line { text, .. } => text.as_ref(),
            other => panic!("{other:?}"),
        })
    };
    model
        .rows()
        .iter()
        .filter_map(|row| match row {
            DisplayRow::Split { old, new } => Some((text(*old), text(*new))),
            DisplayRow::Header(_) | DisplayRow::Notice(_) | DisplayRow::Unified(_) => None,
        })
        .collect()
}

#[test]
fn replacements_pair_by_index_and_unmatched_additions_stay_on_the_new_side() {
    let source = [
        (LineKind::Removed, "before one"),
        (LineKind::Removed, "before two"),
        (LineKind::Added, "after one"),
        (LineKind::Added, "after two"),
        (LineKind::Added, "after three"),
    ];
    let document = model(&source, Layout::Split);
    assert_eq!(
        pairs(&document),
        [
            (Some("before one"), Some("after one")),
            (Some("before two"), Some("after two")),
            (None, Some("after three"))
        ]
    );
}

#[test]
fn context_has_distinct_old_and_new_review_keys_and_headers_have_no_cells() {
    let document = model(&[(LineKind::Context, "same line")], Layout::Split);
    assert!(matches!(document.rows()[0], DisplayRow::Header(_)));
    let DisplayRow::Split {
        old: Some(old),
        new: Some(new),
    } = document.rows()[1]
    else {
        panic!("not a pair")
    };
    assert_eq!(
        document.review_key(old),
        Some(ReviewKey {
            path: "example.ts".parse().unwrap(),
            side: ReviewSide::Old,
            line: SourceLine::new(10).unwrap()
        })
    );
    assert_eq!(
        document.review_key(new),
        Some(ReviewKey {
            path: "example.ts".parse().unwrap(),
            side: ReviewSide::New,
            line: SourceLine::new(20).unwrap()
        })
    );
    assert_eq!(old.source(), new.source());
    assert!(std::ptr::eq(
        document.source(old.source()).unwrap(),
        document.source(new.source()).unwrap()
    ));
}

#[test]
fn unified_targets_keep_removed_old_and_added_or_context_new_source_numbers() {
    let document = model(
        &[
            (LineKind::Context, "before"),
            (LineKind::Removed, "removed"),
            (LineKind::Added, "inserted"),
            (LineKind::Context, "after"),
        ],
        Layout::Unified,
    );
    let targets: Vec<_> = document
        .rows()
        .iter()
        .filter_map(|row| match row {
            DisplayRow::Unified(cell) => Some(document.review_key(*cell).unwrap()),
            DisplayRow::Header(_) | DisplayRow::Notice(_) | DisplayRow::Split { .. } => None,
        })
        .map(|key| (key.side, key.line.get()))
        .collect();
    assert_eq!(
        targets,
        [
            (ReviewSide::New, 20),
            (ReviewSide::Old, 11),
            (ReviewSide::New, 21),
            (ReviewSide::New, 22)
        ]
    );
}

#[test]
fn hunk_boundaries_flush_runs_restart_numbers_and_never_pair_across_hunks() {
    let source: Arc<[DiffRow]> = diff_rows(
        &[
            hunk(75, 75, &[(LineKind::Removed, "old")]),
            hunk(165, 166, &[(LineKind::Added, "new")]),
        ],
        0,
    )
    .into();
    let document = DiffDocument::new(&ticket("src/a.rs"), source.clone(), Layout::Split);
    assert_eq!(pairs(&document), [(Some("old"), None), (None, Some("new"))]);
    let keys: Vec<_> = document
        .rows()
        .iter()
        .filter_map(|row| match row {
            DisplayRow::Split { old, new } => {
                old.or(*new).and_then(|cell| document.review_key(cell))
            }
            DisplayRow::Header(_) | DisplayRow::Notice(_) | DisplayRow::Unified(_) => None,
        })
        .map(|key| key.line.get())
        .collect();
    assert_eq!(keys, [75, 166]);
    let DisplayRow::Header(index) = document.rows()[0] else {
        panic!("no header")
    };
    assert!(std::ptr::eq(document.source(index).unwrap(), &source[0]));
}

#[test]
fn explicit_newline_and_truncation_notices_survive_both_layouts_without_review_targets() {
    let mut hunk = hunk(
        1,
        1,
        &[
            (LineKind::Added, "new"),
            (LineKind::NoNewline, "\\ No newline at end of file"),
        ],
    );
    hunk.new_lines = 2;
    let source: Arc<[DiffRow]> = diff_rows(&[hunk], 0).into();
    for layout in [Layout::Unified, Layout::Split] {
        let document = DiffDocument::new(&ticket("a.rs"), source.clone(), layout);
        let notices = document
            .rows()
            .iter()
            .filter(|row| matches!(row, DisplayRow::Notice(_)))
            .count();
        assert_eq!(notices, 2);
    }
}

#[test]
fn canonical_keys_retain_path_punctuation_and_refuse_source_line_zero() {
    let key = ReviewKey {
        path: "a:old:1/file with spaces.rs".parse().unwrap(),
        side: ReviewSide::New,
        line: SourceLine::new(10).unwrap(),
    };
    let encoded = serde_json::to_string(&key).unwrap();
    assert_eq!(serde_json::from_str::<ReviewKey>(&encoded).unwrap(), key);
    assert!(SourceLine::new(0).is_none());
    assert!(serde_json::from_str::<SourceLine>("0").is_err());
}

#[test]
fn a_retained_cell_cannot_become_a_review_target_in_a_new_file_read() {
    use super::super::{Comparison, PanelReads, RepositoryKey};
    use crate::workspace::git::RequestKeys;
    let root = RepositoryKey {
        node: vessel_proto::NodeId::random(),
        root: vessel_proto::workspace::WorkspaceRoot::Project {
            project: vessel_proto::ProjectId::random(),
        },
    };
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(root, Comparison::default());
    let old = reads.begin_file("same.rs".parse().unwrap(), keys.issue().unwrap());
    let source: Arc<[DiffRow]> = diff_rows(&[hunk(1, 1, &[(LineKind::Context, "same")])], 0).into();
    let before = DiffDocument::new(&old, source.clone(), Layout::Unified);
    let DisplayRow::Unified(cell) = before.rows()[1] else {
        panic!("not a source cell")
    };
    let split_same_read = DiffDocument::new(&old, source.clone(), Layout::Split);
    assert_eq!(before.review_key(cell), split_same_read.review_key(cell));
    let new = reads.begin_file("same.rs".parse().unwrap(), keys.issue().unwrap());
    let after = DiffDocument::new(&new, source, Layout::Unified);
    assert!(
        after.review_key(cell).is_none(),
        "a new read cannot use an old row index as authority"
    );
}
