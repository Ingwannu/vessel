//! Cross-pane/query races through the production read store, not a mock key map.

use super::*;
use crate::workspace::git::{Reading, RequestKeys};
use crate::workspace::problem::Problem;
use vessel_proto::workspace::{ChangeStatus, DiffContent, DiffSummary, FileDiff, WorkspaceRoot};
use vessel_proto::{NodeId, ProjectId};

fn repository() -> RepositoryKey {
    RepositoryKey {
        node: NodeId::random(),
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
    }
}

fn summary() -> DiffSummary {
    DiffSummary {
        from: None,
        files: Vec::new(),
        more: false,
    }
}

fn file() -> FileDiff {
    FileDiff {
        path: "a.rs".parse().unwrap(),
        old_path: None,
        status: ChangeStatus::Modified,
        mode: None,
        content: DiffContent::Unchanged,
        more_from: None,
    }
}

#[test]
fn comparison_and_repository_changes_reject_late_answers_even_after_reopening() {
    let mut keys = RequestKeys::default();
    let root = repository();
    let mut reads = PanelReads::new(root, Comparison::default());
    let old = reads.begin_summary(keys.issue().unwrap());
    reads.select(Comparison::new(
        ComparisonMode::Uncommitted,
        None,
        Whitespace::Show,
    ));
    let current = reads.begin_summary(keys.issue().unwrap());
    assert!(!reads.finish_summary(&old, Ok(summary())));
    assert!(reads.finish_summary(&current, Ok(summary())));
    reads = PanelReads::new(repository(), Comparison::default());
    let other = reads.begin_summary(keys.issue().unwrap());
    assert!(!reads.finish_summary(&old, Ok(summary())));
    assert!(reads.finish_summary(&other, Ok(summary())));
    reads = PanelReads::new(root, Comparison::default());
    let reopened = reads.begin_summary(keys.issue().unwrap());
    assert!(!reads.finish_summary(&old, Ok(summary())));
    assert!(reads.finish_summary(&reopened, Ok(summary())));
}

#[test]
fn an_unsent_read_settles_instead_of_leaving_a_spinner_forever() {
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(repository(), Comparison::default());
    let status = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(status, Err(Problem::Unsent)));
    assert!(matches!(reads.status(), Reading::Failed(Problem::Unsent)));
    let summary = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(&summary, Err(Problem::Unsent)));
    assert!(matches!(reads.summary(), Reading::Failed(Problem::Unsent)));
    let path = "a.rs".parse().unwrap();
    let file = reads.begin_file(path, keys.issue().unwrap());
    assert!(reads.finish_file(&file, Err(Problem::Unsent)));
    assert!(matches!(
        reads.file(file.path()),
        Some(Reading::Failed(Problem::Unsent))
    ));
}

#[test]
fn fresh_summary_invalidates_file_bodies_even_if_the_counts_have_not_changed() {
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(repository(), Comparison::default());
    let first_summary = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(&first_summary, Ok(summary())));
    let old_file = reads.begin_file("a.rs".parse().unwrap(), keys.issue().unwrap());
    assert!(reads.finish_file(&old_file, Ok(file())));
    let refreshed = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(&refreshed, Ok(summary())));
    assert!(reads.file(old_file.path()).is_none());
    let current_file = reads.begin_file("a.rs".parse().unwrap(), keys.issue().unwrap());
    assert!(!reads.finish_file(&old_file, Ok(file())));
    assert!(reads.finish_file(&current_file, Ok(file())));
}
