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
    /// A cancelled metadata caller cannot free a still-blocked worker slot.
    #[error("workspace root metadata admission is busy")]
    MetadataBusy,
    /// Cooperative cancellation before a mutation begins is an explicit refusal.
    #[error("workspace write was cancelled")]
    WriteCancelled,
    /// Shutdown closes new physical writer admission on every platform.
    #[error("workspace writers are stopping")]
    Stopping,
    /// The retained operation failed to reach its caller; never claim success.
    #[error("file operation response was lost")]
    ReplyGone(#[source] tokio::sync::oneshot::error::RecvError),
    /// A retained blocking operation failed; preserve the executor source.
    #[error("file operation worker stopped")]
    Task(#[from] tokio::task::JoinError),
    #[error("physical root filesystem access failed")]
    Directory(#[source] std::io::Error),
    #[cfg(target_os = "linux")]
    #[error("{0}")]
    RootAdmission(#[from] super::exclusion::RootRefusal),
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
            FileError::Kind(..) | FileError::TooLarge => Self::Invalid(error.to_string()),
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
            Self::Directory(_) => ErrorCode::Internal,
            #[cfg(target_os = "linux")]
            Self::RootAdmission(
                super::exclusion::RootRefusal::Busy
                | super::exclusion::RootRefusal::RecoveryPending,
            ) => ErrorCode::Conflict,
            #[cfg(target_os = "linux")]
            Self::RootAdmission(
                super::exclusion::RootRefusal::TimedOut | super::exclusion::RootRefusal::Closed,
            ) => ErrorCode::Unavailable,
            #[cfg(target_os = "linux")]
            Self::RootAdmission(
                super::exclusion::RootRefusal::NotRecovering
                | super::exclusion::RootRefusal::StaleLease,
            ) => ErrorCode::Conflict,
            Self::Stopping | Self::WriteCancelled | Self::MetadataBusy => ErrorCode::Unavailable,
            Self::ReplyGone(_) | Self::Task(_) => ErrorCode::Internal,
            Self::NoGit | Self::NotGit => ErrorCode::Unsupported,
            Self::UnknownProject(_)
            | Self::UnknownWorktree(_)
            | Self::UnknownSnapshot(_)
            | Self::NoSuchPath(_) => ErrorCode::NotFound,
            Self::Invalid(_) => ErrorCode::InvalidRequest,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Busy(_) | Self::Git(GitError::TimedOut { .. } | GitError::Cancelled) => {
                ErrorCode::Unavailable
            }
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
