//! File actions for the code panel's shown comparison, not the commit form.
//!
//! [Decision Log]
//! - Purpose: stage/unstage/discard act on the file whose diff is on screen.
//! - Constraints: both machine and root must match; refreshing facts cannot
//!   authorize a write; the node's discard restores both index and worktree.
//! - Alternatives: infer the repository from a cached session, or offer a
//!   whole-file discard while the person is only viewing one side of it.
//! - Choice: accept the panel's shown repository and comparison; offer both
//!   index actions plus a confirmed discard in Uncommitted, only unstage in
//!   Staged, only stage in Unstaged, and no write in Base.
//! - Why: Paseo's `file-header.tsx`/`file-actions-menu.tsx` place destructive
//!   actions on the file row; `diff-pane.tsx` only offers revert for the
//!   uncommitted view. Native's additional index comparisons stay explicit.
//! - Cost: users choose Uncommitted before discarding the entire local file.

use vessel_proto::workspace::{GitStatus, RelPath, StatusEntry, WorkspaceRoot};

use super::{IndexAction, Reading, Repository, Writing, file_actions, matching_repository};
use crate::workspace::changes::ScopeKind;

/// An index operation or a destructive whole-file decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    /// Stage or unstage this path.
    Index(IndexAction),
    /// Open a confirm for the complete local file, never write immediately.
    Discard,
}

/// A row's complete preparation facts and available intentions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMenu {
    /// The verified shown root.
    pub root: WorkspaceRoot,
    /// Complete status data used to prepare a subsequent decision.
    pub file: StatusEntry,
    /// Intention order, with discard last.
    pub actions: Vec<FileAction>,
    /// A write already owns this repository; the frame stays but has no click.
    pub enabled: bool,
}

/// Prepare only a file present in a current read of the repository on screen.
#[must_use]
pub fn file_menu(
    shown: Option<Repository<'_>>,
    expected: Repository<'_>,
    read: &Reading<GitStatus>,
    writing: &Writing,
    path: &RelPath,
    scope: ScopeKind,
) -> Option<FileMenu> {
    let root = matching_repository(shown, expected)?;
    let file = read.ready()?.files.iter().find(|file| &file.path == path)?;
    let mut actions: Vec<FileAction> = file_actions(file)
        .iter()
        .copied()
        .filter(|action| match scope {
            ScopeKind::Base => false,
            ScopeKind::Uncommitted => true,
            ScopeKind::Staged => *action == IndexAction::Unstage,
            ScopeKind::Unstaged => *action == IndexAction::Stage,
        })
        .map(FileAction::Index)
        .collect();
    if scope == ScopeKind::Uncommitted {
        actions.push(FileAction::Discard);
    }
    if actions.is_empty() {
        return None;
    }
    Some(FileMenu {
        root,
        file: file.clone(),
        actions,
        enabled: !writing.busy(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::git::{RequestKeys, fixtures};
    use vessel_proto::workspace::FileState;

    fn root(id: &str) -> WorkspaceRoot {
        WorkspaceRoot::Project {
            project: id.parse().unwrap(),
        }
    }
    fn file() -> StatusEntry {
        StatusEntry {
            path: "src/한글.rs".parse().unwrap(),
            old_path: None,
            index: FileState::Modified,
            worktree: FileState::Modified,
        }
    }

    #[test]
    fn a_slot_for_a_different_shown_machine_or_root_is_none() {
        let first = root("6f1c2a3e-0000-4000-8000-000000000001");
        let second = root("6f1c2a3e-0000-4000-8000-000000000002");
        let expected = Repository {
            machine: "first",
            root: first,
        };
        let read = Reading::Ready(fixtures::status(vec![file()]));
        for shown in [
            None,
            Some(Repository {
                machine: "second",
                root: first,
            }),
            Some(Repository {
                machine: "first",
                root: second,
            }),
        ] {
            assert_eq!(
                file_menu(
                    shown,
                    expected,
                    &read,
                    &Writing::default(),
                    &file().path,
                    ScopeKind::Uncommitted
                ),
                None
            );
        }
        assert_eq!(
            file_menu(
                Some(expected),
                expected,
                &read,
                &Writing::default(),
                &file().path,
                ScopeKind::Uncommitted
            ),
            Some(FileMenu {
                root: first,
                file: file(),
                enabled: true,
                actions: vec![
                    FileAction::Index(IndexAction::Stage),
                    FileAction::Index(IndexAction::Unstage),
                    FileAction::Discard,
                ],
            })
        );
    }

    #[test]
    fn each_comparison_offers_only_its_own_index_side_and_discard_is_uncommitted_only() {
        let root = root("6f1c2a3e-0000-4000-8000-000000000001");
        let expected = Repository {
            machine: "first",
            root,
        };
        let read = Reading::Ready(fixtures::status(vec![file()]));
        for (scope, actions) in [
            (
                ScopeKind::Staged,
                vec![FileAction::Index(IndexAction::Unstage)],
            ),
            (
                ScopeKind::Unstaged,
                vec![FileAction::Index(IndexAction::Stage)],
            ),
        ] {
            assert_eq!(
                file_menu(
                    Some(expected),
                    expected,
                    &read,
                    &Writing::default(),
                    &file().path,
                    scope
                ),
                Some(FileMenu {
                    root,
                    file: file(),
                    actions,
                    enabled: true,
                })
            );
        }
        assert_eq!(
            file_menu(
                Some(expected),
                expected,
                &read,
                &Writing::default(),
                &file().path,
                ScopeKind::Base
            ),
            None
        );
    }

    #[test]
    fn a_busy_write_keeps_the_frame_disabled_and_a_late_read_cannot_authorize_it() {
        let root = root("6f1c2a3e-0000-4000-8000-000000000001");
        let expected = Repository {
            machine: "first",
            root,
        };
        let mut read = Reading::Ready(fixtures::status(vec![file()]));
        let key = RequestKeys::default().issue().unwrap();
        let mut writing = Writing::default();
        assert!(writing.begin(key));
        assert_eq!(
            file_menu(
                Some(expected),
                expected,
                &read,
                &writing,
                &file().path,
                ScopeKind::Staged
            ),
            Some(FileMenu {
                root,
                file: file(),
                actions: vec![FileAction::Index(IndexAction::Unstage)],
                enabled: false,
            })
        );
        read.start(key);
        assert_eq!(
            file_menu(
                Some(expected),
                expected,
                &read,
                &Writing::default(),
                &file().path,
                ScopeKind::Staged
            ),
            None
        );
        assert_eq!(
            file_menu(
                Some(expected),
                expected,
                &Reading::Ready(fixtures::status(vec![])),
                &Writing::default(),
                &file().path,
                ScopeKind::Staged
            ),
            None
        );
    }
}
