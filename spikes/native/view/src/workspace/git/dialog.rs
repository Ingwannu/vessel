//! Dialog lifetime after a keyed write completes. A refusal keeps the
//! exact reviewed decision on screen; only success returns to its parent.
//!
//! [Decision Log]
//! - Purpose: failures stay beside the decision that was refused.
//! - Constraints: the reply's root and key were accepted already; a
//!   destructive decision must keep its original root/path or stash id.
//! - Alternatives: close on submit and show a toast; rebuild a failed
//!   confirm from whichever row is selected later.
//! - Choice: one pure transition over the typed outcome. Errors keep the
//!   captured dialog; success returns to the parent form or closes it.
//! - Why: the reference keeps refusals in place, and a later selection
//!   must never replace what the person actually reviewed.
//! - Cost: bounded path/stash data is cloned for retry; no file contents
//!   or UI state enter this model.

use super::{BranchMode, DiscardDecision, DropStash, Reading, Success, Written};
use crate::workspace::problem::Problem;
use vessel_proto::workspace::GitStatus;

/// The git form or prepared destructive decision currently shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    /// The message and scope form.
    Commit(Box<GitStatus>),
    /// A create/rename form.
    Branch(BranchMode),
    /// Saved entries.
    Stashes,
    /// Discard exactly the root/path the person reviewed.
    Discard(DiscardDecision),
    /// Delete exactly the pinned stash entry the person reviewed.
    Drop(DropStash),
}

impl Dialog {
    /// Keep the last displayed commit facts with the draft: a failed live read
    /// must disable submission, not erase the form and its edited message.
    #[must_use]
    pub fn commit(status: &GitStatus) -> Self {
        Self::Commit(Box::new(status.clone()))
    }

    /// Update only a commit draft after a keyed status answer was accepted.
    pub fn refresh_commit(&mut self, status: &GitStatus) {
        if let Self::Commit(previous) = self {
            **previous = status.clone();
        }
    }

    /// Current displayed facts, falling back to the retained draft when a read
    /// failed. The read's `ready()` remains the separate authorization gate.
    #[must_use]
    pub fn commit_status<'a>(&'a self, read: &'a Reading<GitStatus>) -> Option<&'a GitStatus> {
        match self {
            Self::Commit(previous) => Some(read.shown().unwrap_or(previous)),
            Self::Branch(_) | Self::Stashes | Self::Discard(_) | Self::Drop(_) => None,
        }
    }

    /// The caller has already accepted the reply's root and request key.
    /// Failures never dismiss the place which explains what was refused.
    #[must_use]
    pub fn after_write(&self, result: &Result<Written, Problem>) -> Option<Self> {
        match result {
            Ok(Written::Status(_)) => match self {
                Self::Discard(_) => None,
                Self::Commit(_) | Self::Branch(_) | Self::Stashes | Self::Drop(_) => {
                    Some(self.clone())
                }
            },
            Ok(Written::Said(
                Success::Committed(_) | Success::Switched(_) | Success::Renamed(_),
            )) => None,
            Ok(Written::Said(Success::Stashed(_))) => Some(match self {
                Self::Drop(_) => Self::Stashes,
                Self::Commit(_) | Self::Branch(_) | Self::Stashes | Self::Discard(_) => {
                    self.clone()
                }
            }),
            Err(_)
            | Ok(
                Written::Refused(_)
                | Written::Said(
                    Success::Fetched
                    | Success::Pushed { .. }
                    | Success::Pulled { .. }
                    | Success::UpToDate,
                ),
            ) => Some(self.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::git::fixtures;
    use vessel_proto::workspace::{
        FileState, GitRefusal, StashEntry, StashList, StatusEntry, WorkspaceRoot,
    };

    #[test]
    fn a_refused_discard_keeps_the_exact_decision_and_success_returns_to_the_form() {
        let root = WorkspaceRoot::Project {
            project: "6f1c2a3e-0000-4000-8000-000000000001".parse().unwrap(),
        };
        let decision = DiscardDecision::of(
            root,
            &StatusEntry {
                path: "src/한글.rs".parse().unwrap(),
                old_path: None,
                index: FileState::Modified,
                worktree: FileState::Modified,
            },
        );
        let dialog = Dialog::Discard(decision);
        assert_eq!(
            dialog.after_write(&Err(Problem::Older)),
            Some(dialog.clone())
        );
        assert_eq!(
            dialog.after_write(&Ok(Written::Refused(GitRefusal::HookFailed {
                output: "entire hook output".to_owned()
            }))),
            Some(dialog.clone())
        );
        assert_eq!(
            dialog.after_write(&Ok(Written::Status(fixtures::status(vec![])))),
            None
        );
    }

    #[test]
    fn dropping_keeps_a_refusal_in_place_and_returns_to_the_refreshed_stash_list() {
        let root = WorkspaceRoot::Project {
            project: "6f1c2a3e-0000-4000-8000-000000000001".parse().unwrap(),
        };
        let decision = DropStash::of(
            root,
            StashEntry {
                index: 7,
                commit: "1111111111111111111111111111111111111111".parse().unwrap(),
                message: "saved".to_owned(),
                time: "2026-11-12T09:00:00Z".parse().unwrap(),
            },
        );
        let dialog = Dialog::Drop(decision);
        assert_eq!(
            dialog.after_write(&Ok(Written::Refused(GitRefusal::StashMoved))),
            Some(dialog.clone())
        );
        assert_eq!(
            dialog.after_write(&Ok(Written::Said(Success::Stashed(StashList {
                entries: vec![],
                more: false
            })))),
            Some(Dialog::Stashes)
        );
    }

    #[test]
    fn a_successful_branch_form_closes_and_unrelated_successes_keep_open_forms() {
        for dialog in [
            Dialog::commit(&fixtures::status(vec![])),
            Dialog::Branch(BranchMode::Create),
            Dialog::Branch(BranchMode::Rename),
            Dialog::Stashes,
        ] {
            assert_eq!(
                dialog.after_write(&Ok(Written::Said(Success::Switched(
                    "feature".parse().unwrap()
                )))),
                None
            );
            assert_eq!(
                dialog.after_write(&Ok(Written::Said(Success::Renamed(
                    "feature".parse().unwrap()
                )))),
                None
            );
            assert_eq!(
                dialog.after_write(&Ok(Written::Said(Success::Fetched))),
                Some(dialog.clone())
            );
            assert_eq!(
                dialog.after_write(&Ok(Written::Status(fixtures::status(vec![])))),
                Some(dialog.clone())
            );
        }
    }
}

#[cfg(test)]
mod draft_tests {
    use super::*;
    use crate::workspace::git::{RequestKeys, fixtures};

    #[test]
    fn a_failed_live_read_keeps_the_last_displayed_draft_but_cannot_authorize_a_commit() {
        let initial = fixtures::status(vec![]);
        let mut dialog = Dialog::commit(&initial);
        let mut current = initial;
        current.branch = Some("updated".parse().unwrap());
        dialog.refresh_commit(&current);
        let failed = Reading::Failed(Problem::Older);
        assert_eq!(dialog.commit_status(&failed), Some(&current));
        assert_eq!(failed.ready(), None);
        let mut refreshing = Reading::default();
        refreshing.start(RequestKeys::default().issue().unwrap());
        assert_eq!(dialog.commit_status(&refreshing), Some(&current));
        assert_eq!(refreshing.ready(), None);
    }
}
