//! Real accepted generations, not content/count equality or a second key book.
use super::*;
use crate::workspace::git::{RequestKeys, fixtures};
use vessel_proto::workspace::{
    ChangeStatus, DiffContent, DiffSummary, FileChange, FileDiff, RelPath, WorkspaceRoot,
};
use vessel_proto::{NodeId, ProjectId};

pub(super) fn reads() -> PanelReads {
    PanelReads::new(
        RepositoryKey {
            node: NodeId::random(),
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
        },
        Comparison::default(),
    )
}
pub(super) fn path() -> RelPath {
    "src/한글 file.rs".parse().unwrap()
}
fn summary() -> DiffSummary {
    DiffSummary {
        from: None,
        files: vec![FileChange {
            path: path(),
            old_path: None,
            status: ChangeStatus::Modified,
            additions: Some(1),
            deletions: Some(1),
            binary: false,
        }],
        more: false,
    }
}
fn file() -> FileDiff {
    FileDiff {
        path: path(),
        old_path: None,
        status: ChangeStatus::Modified,
        mode: None,
        content: DiffContent::Unchanged,
        more_from: None,
    }
}
pub(super) fn load(reads: &mut PanelReads, keys: &mut RequestKeys) -> FileReadTarget {
    let status = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(status, Ok(fixtures::status(Vec::new()))));
    let ticket = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(&ticket, Ok(summary())));
    reads.file_target(&path()).unwrap()
}

#[test]
fn only_a_ready_current_summary_captures_a_file_that_it_actually_contains() {
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    assert!(reads.file_target(&path()).is_none());
    let target = load(&mut reads, &mut keys);
    assert!(reads.file_target_is_current(&target));
    assert!(
        reads
            .file_target(&"not-in-summary.rs".parse().unwrap())
            .is_none()
    );
    let ticket = reads
        .begin_captured_file(&target, keys.issue().unwrap())
        .unwrap();
    assert_eq!(ticket.path(), &path());
    assert_eq!(ticket.comparison().repository(), reads.repository());
    assert!(reads.finish_file(&ticket, Ok(file())));
    assert!(!reads.finish_file(&ticket, Ok(file())));
}

#[test]
fn equal_refresh_data_does_not_revive_the_old_activation_or_its_inflight_body() {
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    let before = load(&mut reads, &mut keys);
    let old_body = reads
        .begin_captured_file(&before, keys.issue().unwrap())
        .unwrap();
    let refresh = reads.begin_summary(keys.issue().unwrap());
    assert_eq!(reads.summary().shown(), Some(&summary()));
    assert!(reads.file_target(&path()).is_none());
    assert!(!reads.file_target_is_current(&before));
    assert!(
        reads
            .begin_captured_file(&before, keys.issue().unwrap())
            .is_none()
    );
    assert!(!reads.finish_file(&old_body, Ok(file())));
    assert!(reads.finish_summary(&refresh, Ok(summary())));
    let after = reads.file_target(&path()).unwrap();
    assert_ne!(before, after);
    assert!(!reads.file_target_is_current(&before));
    assert!(!reads.finish_file(&old_body, Ok(file())));
    let current = reads
        .begin_captured_file(&after, keys.issue().unwrap())
        .unwrap();
    assert!(reads.finish_file(&current, Ok(file())));
}

#[test]
fn comparison_round_trip_and_retirement_never_reuse_the_old_summary_generation() {
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    let original_comparison = reads.comparison().clone();
    let before = load(&mut reads, &mut keys);
    reads.select(Comparison::new(
        ComparisonMode::Uncommitted,
        None,
        Whitespace::Show,
    ));
    reads.select(original_comparison);
    assert!(!reads.file_target_is_current(&before));
    let after = load(&mut reads, &mut keys);
    assert_ne!(before, after);
    assert!(!reads.file_target_is_current(&before));
    assert!(reads.file_target_is_current(&after));
    reads.retire();
    assert!(reads.file_target(&path()).is_none());
    assert!(
        reads
            .begin_captured_file(&after, keys.issue().unwrap())
            .is_none()
    );
    let reopened = load(&mut reads, &mut keys);
    assert_ne!(after, reopened);
    assert!(!reads.file_target_is_current(&after));
}

#[test]
fn failed_or_refreshing_repository_facts_cannot_activate_retained_summary_data() {
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    let target = load(&mut reads, &mut keys);
    let status = reads.begin_status(keys.issue().unwrap());
    assert_eq!(reads.summary().shown(), Some(&summary()));
    assert!(reads.file_target(&path()).is_none());
    assert!(!reads.file_target_is_current(&target));
    assert!(reads.finish_status(status, Err(ReadFailure::DestinationClosed)));
    assert!(reads.file_target(&path()).is_none());
    let status = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(status, Ok(fixtures::status(Vec::new()))));
    assert!(reads.file_target(&path()).is_none());
    let summary = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(&summary, Err(ReadFailure::TooLarge)));
    assert!(reads.file_target(&path()).is_none());
    assert!(
        reads
            .begin_captured_file(&target, keys.issue().unwrap())
            .is_none()
    );
}
