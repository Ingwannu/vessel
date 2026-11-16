//! What a workspace call can fail with, mapped to the wire once.

mod task;

use task::ProjectTaskCause;
use vessel_proto::workspace::RelPath;
use vessel_proto::{ErrorCode, ProjectId, WireError, WorktreeId};
use vessel_store::StoreError;
use vessel_workspace::GitError;
use vessel_workspace::files::FileError;
use vessel_workspace::paths::PathError;

/// A workspace call's failure.
#[derive(Debug, thiserror::Error)]
pub(crate) enum WorkspaceError {
    /// A tracked project/backing operation stopped; retain its actual task cause.
    #[error("the project operation stopped")]
    ProjectTask(#[source] ProjectTaskCause),
    /// The node found no usable git.
    #[error("this machine has no usable git (2.25 or newer)")]
    NoGit,
    /// The project is a folder without git.
    #[error("this project is not a git repository")]
    NotGit,
    /// No such project.
    #[error("no project {0} on this machine")]
    UnknownProject(ProjectId),
    /// No such worktree.
    #[error("no worktree {0} on this machine")]
    UnknownWorktree(WorktreeId),
    /// No snapshot of that worktree.
    #[error("no snapshot of worktree {0}")]
    UnknownSnapshot(WorktreeId),
    /// A path that is not there, or leads out of its folder: one answer
    /// for both, so a caller cannot learn what exists outside (ADR 0016).
    #[error("no such path in this folder: `{0}`")]
    NoSuchPath(RelPath),
    /// The request asks for something the rules refuse.
    #[error("{0}")]
    Invalid(String),
    /// An internal setup plan exceeds the parser's command bound.
    #[error("setup plan exceeds the command bound")]
    SetupPlanTooLarge,
    /// The target is in the wrong state.
    #[error("{0}")]
    Conflict(String),
    /// Too many git runs wait already, or another git holds a lock.
    #[error("{0}")]
    Busy(String),
    /// git failed in a way no refusal covers.
    #[error("git failed")]
    Git(#[source] GitError),
    /// The store refused.
    #[error("the node could not record it")]
    Store(#[source] StoreError),
    /// The disk refused.
    #[error("the disk refused: {0}")]
    Io(String),
}

impl From<GitError> for WorkspaceError {
    fn from(error: GitError) -> Self {
        if matches!(error, GitError::Busy) {
            return Self::Busy(error.to_string());
        }
        // ADR 0030: what a person must change in the repository, said as
        // a conflict with its key (never its value).
        if matches!(
            error,
            GitError::OwnConfig { .. } | GitError::OwnConfigTooLarge
        ) {
            return Self::Conflict(error.to_string());
        }
        if let GitError::Failed { message, .. } = &error
            && message.contains("index.lock")
        {
            return Self::Busy("another git is changing this repository; try again".to_owned());
        }
        Self::Git(error)
    }
}

impl From<StoreError> for WorkspaceError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<PathError> for WorkspaceError {
    fn from(error: PathError) -> Self {
        match error {
            PathError::Outside(path) | PathError::NotFound(path) | PathError::GitDir(path) => {
                Self::NoSuchPath(path)
            }
            PathError::Root => Self::Invalid("the folder itself cannot be the target".to_owned()),
            PathError::Platform(path) => {
                Self::Invalid(format!("`{path}` cannot be used on this machine"))
            }
            PathError::Io(error) => Self::Io(error.to_string()),
        }
    }
}

impl From<FileError> for WorkspaceError {
    fn from(error: FileError) -> Self {
        match error {
            FileError::Path(path) => path.into(),
            FileError::Kind(..)
            | FileError::TooLarge
            | FileError::TransferTooLarge
            | FileError::Offset
            | FileError::CopyLimit
            | FileError::InvalidEntry(_) => Self::Invalid(error.to_string()),
            FileError::Changed(_) | FileError::Exists(_) | FileError::NotEmpty(_) => {
                Self::Conflict(error.to_string())
            }
            FileError::Io(io) => Self::Io(io.to_string()),
            FileError::Task => Self::Io(error.to_string()),
        }
    }
}

impl WorkspaceError {
    /// Seal a tracked task's input-controlled panic payload at its diagnostic boundary.
    pub(crate) fn project_task(cause: tokio::task::JoinError) -> Self {
        Self::ProjectTask(ProjectTaskCause::new(cause))
    }

    /// The wire's form: a code a client branches on, and a message.
    pub(crate) fn wire(&self) -> WireError {
        let code = match self {
            Self::NoGit | Self::NotGit => ErrorCode::Unsupported,
            Self::UnknownProject(_)
            | Self::UnknownWorktree(_)
            | Self::UnknownSnapshot(_)
            | Self::NoSuchPath(_) => ErrorCode::NotFound,
            Self::Invalid(_) | Self::SetupPlanTooLarge => ErrorCode::InvalidRequest,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Busy(_) | Self::Git(GitError::TimedOut { .. }) => ErrorCode::Unavailable,
            Self::Git(_) | Self::Store(_) | Self::Io(_) | Self::ProjectTask(_) => {
                ErrorCode::Internal
            }
        };
        if code == ErrorCode::Internal {
            tracing::warn!(error = %self, source = ?std::error::Error::source(self), "workspace call failed");
        }
        WireError::new(code, self.to_string())
    }
}
