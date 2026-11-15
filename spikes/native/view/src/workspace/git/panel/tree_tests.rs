//! Reference cases from diff-tree.test.ts/diff-order.test.ts, exercised against
//! actual rows. Extra cases preserve unknown counts and non-ASCII ordering.

use super::tree::*;
use std::collections::BTreeSet;
use vessel_proto::workspace::{ChangeStatus, FileChange};

fn file(path: &str, added: u32, removed: u32) -> FileChange {
    FileChange {
        path: path.parse().unwrap(),
        old_path: None,
        status: ChangeStatus::Modified,
        additions: Some(added),
        deletions: Some(removed),
        binary: false,
    }
}

fn make_files(paths: &[&str]) -> Vec<FileChange> {
    paths.iter().map(|path| file(path, 1, 0)).collect()
}

fn labels(rows: &[ChangeTreeRow<'_>]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            ChangeTreeRow::Folder { label, depth, .. } => {
                format!("{}[{label}]", "  ".repeat(depth.get()))
            }
            ChangeTreeRow::File { file, depth, .. } => format!(
                "{}{}",
                "  ".repeat(depth.get()),
                file.path.file_name().unwrap()
            ),
        })
        .collect()
}

#[test]
fn empty_root_files_and_compressed_chains_match_reference_rows() {
    for (input, expected) in [
        (vec![], vec![]),
        (
            vec!["README.md", "package.json"],
            vec!["README.md", "package.json"],
        ),
        (
            vec!["packages/app/src/git/diff-pane.tsx"],
            vec!["[packages/app/src/git]", "  diff-pane.tsx"],
        ),
        (
            vec!["a/b/c/one.ts", "a/b/c/two.ts"],
            vec!["[a/b/c]", "  one.ts", "  two.ts"],
        ),
        (
            vec!["packages/app/a.ts", "packages/server/b.ts"],
            vec![
                "[packages]",
                "  [app]",
                "    a.ts",
                "  [server]",
                "    b.ts",
            ],
        ),
    ] {
        let files = make_files(&input);
        assert_eq!(
            labels(&ChangeTree::new(&files).rows(&BTreeSet::new())),
            expected
        );
    }
}

#[test]
fn compressed_identity_is_the_deepest_full_path_and_survives_a_new_sibling() {
    let files = make_files(&["packages/app/src/git/a.ts"]);
    let tree = ChangeTree::new(&files);
    let collapsed = BTreeSet::from(["packages/app/src/git".parse().unwrap()]);
    assert_eq!(labels(&tree.rows(&collapsed)), ["[packages/app/src/git]"]);
    let files = make_files(&["packages/app/src/git/a.ts", "packages/server/b.ts"]);
    assert_eq!(
        labels(&ChangeTree::new(&files).rows(&collapsed)),
        ["[packages]", "  [app/src/git]", "  [server]", "    b.ts"]
    );
}

#[test]
fn directory_first_order_is_shared_by_the_document_and_keeps_source_indices() {
    let files = make_files(&[
        "src/a.ts",
        "src/z/deep.ts",
        "src/m/mid.ts",
        ".gitlab-ci.yml",
    ]);
    let tree = ChangeTree::new(&files);
    assert_eq!(
        tree.ordered_files()
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        [
            "src/m/mid.ts",
            "src/z/deep.ts",
            "src/a.ts",
            ".gitlab-ci.yml"
        ]
    );
    let indices = tree
        .rows(&BTreeSet::new())
        .into_iter()
        .filter_map(|row| match row {
            ChangeTreeRow::File { index, .. } => Some(index.get()),
            ChangeTreeRow::Folder { .. } => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(indices, [2, 1, 0, 3]);
    let files = make_files(&["bin/stack", "bin/stack/run", "src/a.ts", "src/a/z.ts"]);
    assert_eq!(
        ChangeTree::new(&files)
            .ordered_files()
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["bin/stack/run", "bin/stack", "src/a/z.ts", "src/a.ts"]
    );
}

#[test]
fn collapsed_folder_keeps_all_descendant_totals_and_hides_nested_rows() {
    let files = [file("src/a.ts", 3, 1), file("src/nested/b.ts", 5, 2)];
    let tree = ChangeTree::new(&files);
    let collapsed = BTreeSet::from(["src".parse().unwrap()]);
    let rows = tree.rows(&collapsed);
    assert_eq!(labels(&rows), ["[src]"]);
    let ChangeTreeRow::Folder { totals, .. } = rows.first().unwrap() else {
        panic!();
    };
    assert_eq!(totals.additions.unwrap().get(), 8);
    assert_eq!(totals.deletions.unwrap().get(), 3);
}

#[test]
fn unknown_wire_counts_are_never_drawn_as_zero_or_partial_folder_totals() {
    let mut unknown = file("src/binary", 0, 0);
    unknown.binary = true;
    unknown.additions = None;
    unknown.deletions = None;
    let files = [file("src/a.ts", 3, 1), unknown];
    let tree = ChangeTree::new(&files);
    let rows = tree.rows(&BTreeSet::new());
    let ChangeTreeRow::Folder { totals, .. } = rows.first().unwrap() else {
        panic!();
    };
    assert_eq!(
        *totals,
        ChangeTotals {
            additions: None,
            deletions: None
        }
    );
}

#[test]
fn equal_paths_are_stable_and_unicode_sort_uses_reference_utf16_code_units() {
    let files = [
        file("same.ts", 1, 0),
        file("same.ts", 2, 0),
        file("\u{e000}.ts", 0, 0),
        file("\u{10000}.ts", 0, 0),
    ];
    let tree = ChangeTree::new(&files);
    let ordered = tree.ordered_files();
    assert_eq!(
        ordered
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["same.ts", "same.ts", "\u{10000}.ts", "\u{e000}.ts"]
    );
    assert_eq!(
        ordered
            .iter()
            .map(|file| file.additions)
            .collect::<Vec<_>>(),
        [Some(1), Some(2), Some(0), Some(0)]
    );
}

#[test]
fn maximum_bounded_depth_is_processed_without_recursive_visits() {
    let path = format!("{}x", "a/".repeat(2047));
    let files = [file(&path, 1, 0)];
    let tree = ChangeTree::new(&files);
    let rows = tree.rows(&BTreeSet::new());
    assert_eq!(rows.len(), 2);
    let ChangeTreeRow::Folder { path: identity, .. } = rows.first().unwrap() else {
        panic!();
    };
    assert_eq!(*identity, &files.first().unwrap().path.parent().unwrap());
}
