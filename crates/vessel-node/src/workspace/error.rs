//! What a workspace call can fail with, mapped to the wire once.

use super::comparison::{CacheError, JobError};
use std::sync::Arc;
use vessel_proto::workspace::RelPath;
use vessel_proto::{ErrorCode, ProjectId, WireError, WorktreeId};
use vessel_store::StoreError;
use vessel_workspace::GitError;
use vessel_workspace::files::FileError;
use vessel_workspace::git::ComparisonError;
use vessel_workspace::paths::PathError;

/// A workspace call's failure.
#[derive(Debug, thiserror::Error)]
pub(crate) enum WorkspaceError {
    /// Bounded mutation admission is full or the node is stopping.
    #[error(transparent)]
    GitWriteAdmission(#[from] super::git_writes::WriteAdmission),
    /// Waiting never permits following a replacement registered directory.
    #[error("the repository changed while waiting; refresh before trying again")]
    GitTargetChanged,
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
    /// Private comparison index preparation or cleanup failed.
    #[error("the comparison could not be completed")]
    Comparison(#[source] ComparisonError),
    /// Classified history metadata failed without dropping its typed cause.
    #[error("the branch history could not be read")]
    History(#[source] vessel_workspace::git::HistoryError),
    /// Comparison admission, shutdown or reply delivery failed.
    #[error("{0}")]
    ComparisonJob(#[from] JobError),
    /// Cache admission, typed payload validation or shared delivery failed.
    #[error(transparent)]
    ComparisonCache(#[from] CacheError),
    /// Identical readers retain one typed failure and its source chain.
    #[error("{0}")]
    SharedComparison(#[source] Arc<WorkspaceError>),
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

impl From<ComparisonError> for WorkspaceError {
    fn from(error: ComparisonError) -> Self {
        match error {
            ComparisonError::Git(GitError::Unexpected { what: "base" }) => {
                Self::Invalid("the requested comparison base does not resolve".to_owned())
            }
            ComparisonError::Git(error) => Self::from(error),
            error @ (ComparisonError::Io(_)
            | ComparisonError::Task(_)
            | ComparisonError::Utf8(_)
            | ComparisonError::Object(_)
            | ComparisonError::Path(_)
            | ComparisonError::Cleanup { .. }
            | ComparisonError::CleanupTask { .. }) => Self::Comparison(error),
        }
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
    /// The wire's form: a code a client branches on, and a message.
    pub(crate) fn wire(&self) -> WireError {
        let code = match self {
            Self::NoGit | Self::NotGit => ErrorCode::Unsupported,
            Self::UnknownProject(_)
            | Self::UnknownWorktree(_)
            | Self::UnknownSnapshot(_)
            | Self::NoSuchPath(_) => ErrorCode::NotFound,
            Self::Invalid(_) => ErrorCode::InvalidRequest,
            Self::Conflict(_) | Self::GitTargetChanged => ErrorCode::Conflict,
            Self::Busy(_)
            | Self::GitWriteAdmission(_)
            | Self::Git(GitError::TimedOut { .. })
            | Self::ComparisonJob(JobError::Busy | JobError::Closed)
            | Self::ComparisonCache(CacheError::Refused(_)) => ErrorCode::Unavailable,
            Self::Git(_)
            | Self::Store(_)
            | Self::Io(_)
            | Self::Comparison(_)
            | Self::History(_)
            | Self::ComparisonJob(JobError::Worker(_))
            | Self::ComparisonCache(
                CacheError::Channel(_) | CacheError::Payload { .. } | CacheError::Ownership,
            ) => ErrorCode::Internal,
            Self::SharedComparison(error) => return error.wire(),
        };
        if code == ErrorCode::Internal {
            tracing::warn!(error = %self, source = ?std::error::Error::source(self), "workspace call failed");
        }
        WireError::new(code, self.to_string())
    }
}
