//! Pending labels describe the request that owns the keyed write, not an
//! action guessed from the latest status or Git's output text.

use vessel_proto::workspace::{RelPath, StashEntry};

use super::{Action, FileAction, IndexAction, StashChoice, StashScope};
use crate::workspace::git_quick::GitAction;

/// The exact intention whose pending state a menu may show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteIntent {
    /// Existing non-visual tracker callers do not need an action label.
    Unspecified,
    /// Commit, push, pull or fetch.
    Quick(GitAction),
    /// A listed branch, including remote-to-local checkout.
    SwitchBranch,
    /// A named branch created from HEAD.
    CreateBranch,
    /// Rename the checked-out branch.
    RenameBranch,
    /// Save local changes with the chosen untracked scope.
    SaveStash(StashScope),
    /// The saved entry remains pinned by both index and commit.
    Stash {
        /// Apply, pop or drop.
        choice: StashChoice,
        /// Complete listed facts, never just a movable index.
        entry: StashEntry,
    },
    /// Stage or unstage one displayed path.
    Index {
        /// Which index operation.
        action: IndexAction,
        /// The displayed path.
        path: RelPath,
    },
    /// A confirmed complete local file.
    Discard(RelPath),
}

impl WriteIntent {
    /// The toolbar menu row whose leading glyph becomes its spinner.
    #[must_use]
    pub const fn action(&self) -> Option<Action> {
        match self {
            Self::Quick(action) => Some(Action::Quick(*action)),
            Self::SwitchBranch => Some(Action::SwitchBranch),
            Self::CreateBranch => Some(Action::CreateBranch),
            Self::RenameBranch => Some(Action::RenameBranch),
            Self::SaveStash(scope) => Some(Action::SaveStash(*scope)),
            Self::Unspecified | Self::Stash { .. } | Self::Index { .. } | Self::Discard(_) => None,
        }
    }

    /// Another entry at the same index must never acquire this pending state.
    #[must_use]
    pub fn matches_stash(&self, choice: StashChoice, entry: &StashEntry) -> bool {
        match self {
            Self::Stash {
                choice: own,
                entry: pinned,
            } => *own == choice && pinned.index == entry.index && pinned.commit == entry.commit,
            Self::Unspecified
            | Self::Quick(_)
            | Self::SwitchBranch
            | Self::CreateBranch
            | Self::RenameBranch
            | Self::SaveStash(_)
            | Self::Index { .. }
            | Self::Discard(_) => false,
        }
    }

    /// Pending index/discard feedback belongs to one path and intention only.
    #[must_use]
    pub fn matches_file(&self, action: FileAction, path: &RelPath) -> bool {
        match (self, action) {
            (
                Self::Index {
                    action: own,
                    path: own_path,
                },
                FileAction::Index(action),
            ) => *own == action && own_path == path,
            (Self::Discard(own_path), FileAction::Discard) => own_path == path,
            (Self::Index { .. }, FileAction::Discard)
            | (Self::Discard(_), FileAction::Index(_))
            | (
                Self::Unspecified
                | Self::Quick(_)
                | Self::SwitchBranch
                | Self::CreateBranch
                | Self::RenameBranch
                | Self::SaveStash(_)
                | Self::Stash { .. },
                _,
            ) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_feedback_never_moves_to_another_path_or_stash_entry() {
        let path: RelPath = "src/a.rs".parse().unwrap();
        let intent = WriteIntent::Index {
            action: IndexAction::Stage,
            path: path.clone(),
        };
        assert!(intent.matches_file(FileAction::Index(IndexAction::Stage), &path));
        assert!(!intent.matches_file(FileAction::Index(IndexAction::Unstage), &path));
        assert!(!intent.matches_file(
            FileAction::Index(IndexAction::Stage),
            &"src/b.rs".parse().unwrap()
        ));
        let entry = StashEntry {
            index: 1,
            message: "saved".into(),
            commit: "1111111111111111111111111111111111111111".parse().unwrap(),
            time: "2026-01-01T00:00:00Z".parse().unwrap(),
        };
        let intent = WriteIntent::Stash {
            choice: StashChoice::Pop,
            entry: entry.clone(),
        };
        assert!(intent.matches_stash(StashChoice::Pop, &entry));
        let mut moved = entry.clone();
        moved.index = 2;
        assert!(!intent.matches_stash(StashChoice::Pop, &moved));
        moved = entry;
        moved.commit = "2222222222222222222222222222222222222222".parse().unwrap();
        assert!(!intent.matches_stash(StashChoice::Pop, &moved));
        assert_eq!(
            WriteIntent::Quick(GitAction::Push).action(),
            Some(Action::Quick(GitAction::Push))
        );
        assert_eq!(intent.action(), None);
    }
}
