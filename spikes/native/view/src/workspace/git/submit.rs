//! The same submit gate for a button and its keyboard shortcut. A
//! disabled button must not be bypassed by Enter from its field.
//!
//! [Decision Log]
//! - Purpose: Enter and a button must enforce the same form rules.
//! - Constraints: fields are editable text until the wire's constructor
//!   validates them; stale reads and a busy write cannot authorize work.
//! - Alternatives: disable only the drawn button, or send invalid input
//!   to the node and wait for its refusal.
//! - Choice: pure submit gates use the existing availability and wire
//!   constructors; both input paths call them before preparing a write.
//! - Why: the page's empty/staged-only gates hold for keyboard use too.
//! - Cost: edits can revalidate text; the node still checks the request.

use super::{Action, BranchMode, CommitForm, Reading, Writing, availability, branch_request};
use crate::workspace::git_quick::GitAction;
use vessel_proto::workspace::{CommitStage, GitStatus};

/// Whether this commit form may submit; parsing still reports the exact
/// invalid-message reason before a request can reach the link.
#[must_use]
pub fn can_commit(
    status: &Reading<GitStatus>,
    writing: &Writing,
    message: &str,
    stage: CommitStage,
) -> bool {
    if availability(Action::Quick(GitAction::Commit), status, writing).is_some()
        || message.trim().is_empty()
    {
        return false;
    }
    status
        .ready()
        .is_some_and(|status| CommitForm::of(status).has_changes(stage))
}

/// The branch field's Enter and the button share validation and the
/// current repository gate, including the first-commit requirement.
#[must_use]
pub fn can_branch(
    status: &Reading<GitStatus>,
    writing: &Writing,
    mode: BranchMode,
    name: &str,
    root: vessel_proto::workspace::WorkspaceRoot,
) -> bool {
    let action = match mode {
        BranchMode::Create => Action::CreateBranch,
        BranchMode::Rename => Action::RenameBranch,
    };
    availability(action, status, writing).is_none() && branch_request(root, mode, name).is_ok()
}

#[cfg(test)]
mod tests {
    use super::super::{RequestKeys, fixtures};
    use super::*;
    use vessel_proto::workspace::{FileState, StatusEntry, WorkspaceRoot};

    fn changed() -> GitStatus {
        let mut status = fixtures::status(vec![StatusEntry {
            path: "a".parse().unwrap(),
            old_path: None,
            index: FileState::Unmodified,
            worktree: FileState::Modified,
        }]);
        status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
        status
    }

    #[test]
    fn buttons_and_shortcuts_share_the_pages_empty_message_and_scope_gates() {
        let mut status = Reading::Ready(changed());
        let writing = Writing::default();
        for message in ["", " \n\t"] {
            assert!(!can_commit(
                &status,
                &writing,
                message,
                CommitStage::Everything
            ));
        }
        assert!(!can_commit(
            &status,
            &writing,
            "Message",
            CommitStage::Staged
        ));
        assert!(can_commit(
            &status,
            &writing,
            "Message",
            CommitStage::Tracked
        ));
        assert!(can_commit(
            &status,
            &writing,
            "Message",
            CommitStage::Everything
        ));
        let mut staged = changed();
        staged.files.first_mut().unwrap().index = FileState::Modified;
        status = Reading::Ready(staged);
        assert!(can_commit(
            &status,
            &writing,
            "Message",
            CommitStage::Staged
        ));
        let mut keys = RequestKeys::default();
        status.start(keys.issue().unwrap());
        assert!(!can_commit(
            &status,
            &writing,
            "Message",
            CommitStage::Everything
        ));
    }

    #[test]
    fn branch_submit_uses_the_same_parser_before_buttons_or_enter_can_write() {
        let root = WorkspaceRoot::Project {
            project: "6f1c2a3e-0000-4000-8000-000000000001".parse().unwrap(),
        };
        let status = Reading::Ready(changed());
        let mut writing = Writing::default();
        for mode in [BranchMode::Create, BranchMode::Rename] {
            for invalid in ["", " ", "bad name", "topic..branch"] {
                assert!(!can_branch(&status, &writing, mode, invalid, root));
            }
            assert!(can_branch(&status, &writing, mode, " feature/new ", root));
        }
        let mut keys = RequestKeys::default();
        assert!(writing.begin(keys.issue().unwrap()));
        assert!(!can_branch(
            &status,
            &writing,
            BranchMode::Create,
            "feature/new",
            root
        ));
    }
}
