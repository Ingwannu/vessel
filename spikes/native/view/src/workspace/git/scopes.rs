//! The node's three commit scopes, offered through the shared select.

use vessel_proto::workspace::{CommitStage, GitStatus};

use super::{CommitForm, Reading, Writing};

/// One complete scope label's typed value and availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitScope {
    /// What the node stages before committing.
    pub stage: CommitStage,
    /// A current read permits selecting this scope.
    pub enabled: bool,
}

/// A complete empty selected scope cannot be chosen; partial zero remains
/// unknown. All choices and submit share the same derived eligibility and wait
/// for a current read and the repository's previous write.
#[must_use]
pub fn commit_scopes(read: &Reading<GitStatus>, writing: &Writing) -> [CommitScope; 3] {
    let form = read.ready().map(CommitForm::of);
    let ready = form.is_some() && !writing.busy();
    [
        CommitStage::Staged,
        CommitStage::Tracked,
        CommitStage::Everything,
    ]
    .map(|stage| CommitScope {
        stage,
        enabled: ready && form.is_some_and(|form| form.has_changes(stage)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::git::{RequestKeys, fixtures};
    use vessel_proto::workspace::{FileState, StatusEntry};

    fn expected(staged: bool, other: bool) -> [CommitScope; 3] {
        [
            CommitScope {
                stage: CommitStage::Staged,
                enabled: staged,
            },
            CommitScope {
                stage: CommitStage::Tracked,
                enabled: other,
            },
            CommitScope {
                stage: CommitStage::Everything,
                enabled: other,
            },
        ]
    }

    #[test]
    fn the_three_scopes_keep_their_order_and_only_a_nonempty_index_enables_staged() {
        let empty = Reading::Ready(fixtures::status(vec![]));
        assert_eq!(
            commit_scopes(&empty, &Writing::default()),
            expected(false, false)
        );
        let staged = Reading::Ready(fixtures::status(vec![StatusEntry {
            path: "src/a.rs".parse().unwrap(),
            old_path: None,
            index: FileState::Modified,
            worktree: FileState::Unmodified,
        }]));
        assert_eq!(
            commit_scopes(&staged, &Writing::default()),
            expected(true, true)
        );
    }

    #[test]
    fn refresh_and_a_pending_write_leave_every_scope_unavailable() {
        let mut read = Reading::Ready(fixtures::status(vec![]));
        let key = RequestKeys::default().issue().unwrap();
        read.start(key);
        assert_eq!(
            commit_scopes(&read, &Writing::default()),
            expected(false, false)
        );
        let mut writing = Writing::default();
        assert!(writing.begin(key));
        assert_eq!(
            commit_scopes(&Reading::Ready(fixtures::status(vec![])), &writing),
            expected(false, false)
        );
        assert_eq!(
            commit_scopes(&Reading::Empty, &Writing::default()),
            expected(false, false)
        );
    }
}
