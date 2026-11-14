//! The one change a commit form's row names. GOAT's commit form shows a
//! path and what the commit takes from it; the node's status carries two
//! sides per path (index and working tree), so the row names the side the
//! chosen scope commits, in one word, rather than both sides' sentence.

use vessel_proto::workspace::{CommitStage, FileState, StatusEntry};

/// What committing `file` with `stage` takes from it: the index's change
/// when only staged changes go in; otherwise the working tree's, which is
/// newer, unless the working tree matches the index.
#[must_use]
pub const fn committed_state(file: &StatusEntry, stage: CommitStage) -> FileState {
    match (stage, file.worktree) {
        (CommitStage::Staged, _)
        | (CommitStage::Tracked | CommitStage::Everything, FileState::Unmodified) => file.index,
        (CommitStage::Tracked | CommitStage::Everything, worktree) => worktree,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: FileState, worktree: FileState) -> StatusEntry {
        StatusEntry {
            path: "src/a.rs".parse().unwrap(),
            old_path: None,
            index,
            worktree,
        }
    }

    #[test]
    fn staged_only_names_the_index_change() {
        let file = entry(FileState::Added, FileState::Modified);
        assert_eq!(
            committed_state(&file, CommitStage::Staged),
            FileState::Added
        );
    }

    #[test]
    fn a_wider_scope_names_the_working_tree_change() {
        let file = entry(FileState::Added, FileState::Modified);
        assert_eq!(
            committed_state(&file, CommitStage::Tracked),
            FileState::Modified
        );
        let new = entry(FileState::Untracked, FileState::Untracked);
        assert_eq!(
            committed_state(&new, CommitStage::Everything),
            FileState::Untracked
        );
    }

    #[test]
    fn a_working_tree_matching_the_index_names_the_index_change() {
        let file = entry(FileState::Deleted, FileState::Unmodified);
        assert_eq!(
            committed_state(&file, CommitStage::Everything),
            FileState::Deleted
        );
        assert_eq!(
            committed_state(&file, CommitStage::Tracked),
            FileState::Deleted
        );
    }
}
