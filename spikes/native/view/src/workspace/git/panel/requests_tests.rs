//! Cross-pane/query races through the production read store, not a mock key map.

use super::*;
use crate::workspace::git::RequestKeys;
use crate::workspace::problem::Problem;
use vessel_proto::workspace::{
    ChangeStatus, DiffContent, DiffSummary, FileDiff, WorkspaceRoot, WorktreeBase,
};
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
    assert!(reads.finish_status(status, Err(ReadFailure::Call(Problem::Unsent))));
    assert!(matches!(
        reads.status(),
        PanelRead::Failed(ReadFailure::Call(Problem::Unsent))
    ));
    let summary = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(&summary, Err(ReadFailure::Call(Problem::Unsent))));
    assert!(matches!(
        reads.summary(),
        PanelRead::Failed(ReadFailure::Call(Problem::Unsent))
    ));
    let path = "a.rs".parse().unwrap();
    let file = reads.begin_file(path, keys.issue().unwrap());
    assert!(reads.finish_file(&file, Err(ReadFailure::Call(Problem::Unsent))));
    assert!(matches!(
        reads.file(file.path()),
        Some(PanelRead::Failed(ReadFailure::Call(Problem::Unsent)))
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

#[test]
fn another_paths_body_settles_failed_and_cannot_replace_the_requested_file() {
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(repository(), Comparison::default());
    let ticket = reads.begin_file("expected.rs".parse().unwrap(), keys.issue().unwrap());
    assert!(reads.finish_file(&ticket, Ok(file())));
    assert!(
        matches!(reads.file(ticket.path()), Some(PanelRead::Failed(ReadFailure::UnexpectedFile {
        requested, received
    })) if requested.as_str() == "expected.rs" && received.as_str() == "a.rs")
    );
    assert!(
        !reads.finish_file(&ticket, Ok(file())),
        "a duplicate cannot replace the failure"
    );
}

#[test]
fn retiring_the_connection_settles_reads_and_late_answers_cannot_reauthorize_their_facts() {
    let mut keys = RequestKeys::default();
    let root = repository();
    let mut reads = PanelReads::new(root, Comparison::default());
    let old = reads.begin_summary(keys.issue().unwrap());
    reads.retire();
    assert!(matches!(
        reads.status(),
        PanelRead::Failed(ReadFailure::DestinationChanged)
    ));
    assert!(matches!(
        reads.summary(),
        PanelRead::Failed(ReadFailure::DestinationChanged)
    ));
    assert!(!reads.finish_summary(&old, Ok(summary())));
    let new = reads.begin_summary(keys.issue().unwrap());
    assert!(!reads.finish_summary(&old, Ok(summary())));
    assert!(reads.finish_summary(&new, Ok(summary())));
    assert!(reads.summary().ready().is_some());
    let status = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(
        status,
        Err(ReadFailure::UnexpectedNode {
            expected: root.node,
            received: NodeId::random(),
        })
    ));
    assert!(reads.status().ready().is_none());
}

#[test]
fn reader_projection_keeps_status_failure_ahead_of_a_retained_diff() {
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(repository(), Comparison::default());
    assert_eq!(reads.body(Demand::Active), Body::CheckingRepository);
    let first = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(first, Ok(crate::workspace::git::fixtures::status())));
    assert_eq!(reads.body(Demand::Active), Body::ReadingDiff);
    let ticket = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(&ticket, Ok(summary())));
    assert_eq!(reads.body(Demand::Active), Body::Empty(None));
    let next = reads.begin_summary(keys.issue().unwrap());
    assert_eq!(reads.body(Demand::Active), Body::Empty(None));
    let status = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(status, Err(ReadFailure::Call(Problem::Unsent))));
    assert!(reads.finish_summary(&next, Ok(summary())));
    assert_eq!(
        reads.body(Demand::Active),
        Body::RepositoryFailed(&ReadFailure::Call(Problem::Unsent))
    );
    assert_eq!(reads.body(Demand::Inactive), Body::Inactive);
    assert_eq!(reads.body(Demand::Disconnected), Body::Connecting);
}

#[test]
fn explicit_refresh_reason_and_nondata_comparison_results_keep_their_meaning() {
    use vessel_proto::workspace::GitReadReason;
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(repository(), Comparison::default());
    let status = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(status, Ok(crate::workspace::git::fixtures::status())));
    let fresh = reads.begin_summary_for(keys.issue().unwrap(), GitReadReason::Refresh);
    assert_eq!(fresh.reason(), GitReadReason::Refresh);
    assert!(reads.finish_summary(&fresh, Err(ReadFailure::NoBase)));
    assert_eq!(
        reads.body(Demand::Active),
        Body::DiffFailed(&ReadFailure::NoBase)
    );
    let bounded = reads.begin_summary(keys.issue().unwrap());
    assert_eq!(bounded.reason(), GitReadReason::View);
    assert!(reads.finish_summary(&bounded, Err(ReadFailure::TooLarge)));
    assert_eq!(reads.body(Demand::Active), Body::DiffTooLarge);
    let file = reads.begin_file("a.rs".parse().unwrap(), keys.issue().unwrap());
    assert_eq!(file.comparison().reason(), GitReadReason::View);
}

#[test]
fn canonical_queries_keep_all_and_committed_distinct_without_legacy_substitution() {
    use vessel_proto::workspace::{CompareGit, GitComparison, GitReadReason, GitWhitespace};
    let root = repository().root;
    let base = WorktreeBase::Ref {
        name: "refs/remotes/origin/main".parse().unwrap(),
    };
    for (mode, wire) in [
        (
            ComparisonMode::All,
            GitComparison::All {
                base: Some(base.clone()),
            },
        ),
        (
            ComparisonMode::Committed,
            GitComparison::Committed {
                base: Some(base.clone()),
            },
        ),
        (ComparisonMode::Uncommitted, GitComparison::Uncommitted),
    ] {
        let comparison = Comparison::new(mode, Some(base.clone()), Whitespace::Ignore);
        assert_eq!(
            comparison.query(root, GitReadReason::Refresh),
            CompareGit {
                root,
                comparison: wire,
                whitespace: GitWhitespace::Ignore,
                reason: GitReadReason::Refresh,
            }
        );
    }
}
