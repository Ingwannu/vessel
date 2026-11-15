//! ADR 0077 Linux admitted-worker filesystem mechanism; no auth/store/session orchestration.
//!
//! Captures use strict Git enumeration and regular-file guards. The caller
//! owns workspace exclusion and runs synchronous I/O off its async worker.

#[cfg(target_os = "linux")]
mod account;
mod api;
#[cfg(target_os = "linux")]
mod archive;
#[cfg(target_os = "linux")]
mod capture;
#[cfg(target_os = "linux")]
mod content;
mod enumeration;
mod fingerprint;
#[cfg(target_os = "linux")]
mod inspect;
#[cfg(target_os = "linux")]
mod policy;
#[cfg(target_os = "linux")]
mod restore;
#[cfg(target_os = "linux")]
mod rooted;
#[cfg(target_os = "linux")]
mod scope;
mod storage;

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
/// Source-preserving filesystem failure; node adapters expose only its reason.
pub enum Error {
    /// An actual filesystem syscall failed.
    #[error("checkpoint filesystem access failed")]
    Io(#[from] std::io::Error),
    /// Git could not enumerate or recheck the requested scope.
    #[error("checkpoint git enumeration failed")]
    Git(#[from] crate::GitError),
    /// The typed scope/metadata/identity policy refused this operation.
    #[error("checkpoint operation refused: {0}")]
    Refused(#[from] vessel_proto::checkpoint::CheckpointError),
}

pub use api::{Archive, IgnoreDefaults, Restore, Tree, image_fingerprint};
pub use restore::{RecoveryState, Undo as Rollback};
pub use storage::{CaptureStorage, StageDirectory, Storage};

impl Error {
    /// Classify once without putting paths or file contents into RPC errors.
    #[must_use]
    pub fn reason(&self) -> vessel_proto::checkpoint::CheckpointError {
        use vessel_proto::checkpoint::CheckpointError;
        match self {
            Self::Refused(reason) => *reason,
            Self::Io(error) => {
                if error.kind() == std::io::ErrorKind::StorageFull {
                    CheckpointError::StorageFull
                } else {
                    CheckpointError::Unknown
                }
            }
            Self::Git(_) => CheckpointError::Unknown,
        }
    }
}
