//! A commit's scope and message, and which index actions a file offers.

use vessel_proto::workspace::{
    Commit, CommitStage, FileState, GitStatus, InvalidText, StatusEntry, WorkspaceRoot,
};

/// A count of changed files, never a byte or line count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChangedFiles(usize);

impl ChangedFiles {
    /// The number to display.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// A commit dialog's initial scope and summary (the page's commit-dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitForm {
    /// Number of changed files in this status page.
    pub changed: ChangedFiles,
    /// Number with an index change, untracked files excluded.
    pub staged: ChangedFiles,
    /// Staged only when there is an index change, everything otherwise.
    pub stage: CommitStage,
}

impl CommitForm {
    /// Make the initial form from the current status, without changing it.
    #[must_use]
    pub fn of(status: &GitStatus) -> Self {
        let staged = status
            .files
            .iter()
            .filter(|file| staged(file.index))
            .count();
        Self {
            changed: ChangedFiles(status.files.len()),
            staged: ChangedFiles(staged),
            stage: if staged == 0 {
                CommitStage::Everything
            } else {
                CommitStage::Staged
            },
        }
    }
}

/// The two actions on an index, distinguished from destructive discard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexAction {
    /// Take the working copy into the index.
    Stage,
    /// Take the last commit into the index, keeping the working copy.
    Unstage,
}

/// Available index actions in a status row, stage first, then unstage.
/// A file modified both before and after staging offers both actions.
#[must_use]
pub fn file_actions(file: &StatusEntry) -> &'static [IndexAction] {
    match (changed(file.worktree), staged(file.index)) {
        (false, false) => &[],
        (true, false) => &[IndexAction::Stage],
        (false, true) => &[IndexAction::Unstage],
        (true, true) => &[IndexAction::Stage, IndexAction::Unstage],
    }
}

fn staged(state: FileState) -> bool {
    match state {
        FileState::Unmodified | FileState::Untracked => false,
        FileState::Modified
        | FileState::Added
        | FileState::Deleted
        | FileState::Renamed
        | FileState::Copied
        | FileState::TypeChanged
        | FileState::Conflicted
        | FileState::Unknown => true,
    }
}

fn changed(state: FileState) -> bool {
    match state {
        FileState::Unmodified => false,
        FileState::Modified
        | FileState::Added
        | FileState::Deleted
        | FileState::Renamed
        | FileState::Copied
        | FileState::TypeChanged
        | FileState::Untracked
        | FileState::Conflicted
        | FileState::Unknown => true,
    }
}

/// Prepare the page's trimmed message through the wire's constructor.
/// Empty, NUL and oversized text cannot become a request.
///
/// # Errors
/// Returns the wire's validation error, including its precise reason.
pub fn commit_request(
    root: WorkspaceRoot,
    message: &str,
    stage: CommitStage,
) -> Result<Commit, InvalidText> {
    Ok(Commit {
        root,
        message: message.trim().parse()?,
        stage,
    })
}
