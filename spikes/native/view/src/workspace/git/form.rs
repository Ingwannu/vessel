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

/// Whether a bounded status page contains every changed path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCoverage {
    /// Zero eligible paths proves the selected scope is empty.
    Complete,
    /// Omitted paths may contain changes eligible for any scope.
    Partial,
}

/// A commit dialog's initial scope and summary (the page's commit-dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitForm {
    /// Number of changed files in this status page.
    pub changed: ChangedFiles,
    /// Number with an index change, untracked files excluded.
    pub staged: ChangedFiles,
    /// Changed tracked paths, including already-staged additions.
    pub tracked: ChangedFiles,
    /// Whether an absent eligible path is known to be absent.
    pub coverage: StatusCoverage,
    /// Staged only when there is an index change, everything otherwise.
    pub stage: CommitStage,
}

impl CommitForm {
    /// Make the initial form from the current status, without changing it.
    #[must_use]
    pub fn of(status: &GitStatus) -> Self {
        let staged = commit_files(status, CommitStage::Staged).count();
        Self {
            changed: ChangedFiles(status.files.len()),
            staged: ChangedFiles(staged),
            tracked: ChangedFiles(commit_files(status, CommitStage::Tracked).count()),
            coverage: if status.more {
                StatusCoverage::Partial
            } else {
                StatusCoverage::Complete
            },
            stage: if staged == 0 {
                CommitStage::Everything
            } else {
                CommitStage::Staged
            },
        }
    }

    /// Known eligible count, derived once from the status. Tracked uses the
    /// node's `git add --update` semantics, which excludes new untracked paths
    /// while retaining staged additions. The count is a lower bound if partial.
    #[must_use]
    pub const fn eligible(self, stage: CommitStage) -> ChangedFiles {
        match stage {
            CommitStage::Staged => self.staged,
            CommitStage::Tracked => self.tracked,
            CommitStage::Everything => self.changed,
        }
    }

    /// A complete zero disables both choice and submit; a partial zero is
    /// unknown, so the authoritative node gets to decide instead of the client
    /// claiming that omitted tracked/staged changes do not exist.
    #[must_use]
    pub fn has_changes(self, stage: CommitStage) -> bool {
        self.coverage == StatusCoverage::Partial || self.eligible(stage).get() > 0
    }
}

/// The same scope rule supplies both its eligible count and the form's path
/// preview. The complete status remains retained; omitted paths stay unknown.
pub fn commit_files(status: &GitStatus, stage: CommitStage) -> impl Iterator<Item = &StatusEntry> {
    status.files.iter().filter(move |file| match stage {
        CommitStage::Staged => staged_path(file.index),
        CommitStage::Tracked => {
            staged_path(file.index)
                || (changed(file.worktree) && file.worktree != FileState::Untracked)
        }
        CommitStage::Everything => true,
    })
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
    match (changed(file.worktree), staged_path(file.index)) {
        (false, false) => &[],
        (true, false) => &[IndexAction::Stage],
        (false, true) => &[IndexAction::Unstage],
        (true, true) => &[IndexAction::Stage, IndexAction::Unstage],
    }
}

fn staged_path(state: FileState) -> bool {
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
