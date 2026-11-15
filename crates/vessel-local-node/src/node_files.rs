//! The sole Store facade consumer; never opens SQLite or trusts an owner PID.

use std::path::{Path, PathBuf};
use vessel_client::{BearerToken, LocalCredential};
use vessel_proto::NodeId;
use vessel_store::{DataDir, StartLock, StoreError};

/// Explicit private node home; libraries never discover HOME or environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeHome(PathBuf);

/// File errors retain their cause without including token contents.
#[derive(Debug, thiserror::Error)]
pub enum FilesError {
    /// Relative homes would depend on an unrelated process working directory.
    #[error("local node home must be absolute")]
    Relative,
    /// A data-directory operation failed.
    #[error("local node files could not be accessed")]
    Store(#[source] StoreError),
    /// A malformed stored credential cannot authenticate any node.
    #[error("invalid local node credential")]
    Credential(#[source] vessel_client::InvalidToken),
    /// Blocking file work was cancelled or panicked.
    #[error("local node file worker stopped")]
    Worker(#[source] tokio::task::JoinError),
    /// Another launcher owns the serialized start operation.
    #[error("local node start is already in progress")]
    Busy,
}

pub(crate) struct Identity {
    pub(crate) node: NodeId,
    pub(crate) endpoint: Endpoint,
    pub(crate) credential: LocalCredential,
}

pub(crate) struct Endpoint(PathBuf);
impl Endpoint {
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

/// Keeping this guard alive serializes cooperating starts without PID authority.
pub(crate) struct StartPermit {
    _lock: StartLock,
}

impl NodeHome {
    /// Validate the caller-supplied path before any file access or launch.
    ///
    /// # Errors
    /// Relative paths are refused.
    pub fn new(path: PathBuf) -> Result<Self, FilesError> {
        if !path.is_absolute() {
            return Err(FilesError::Relative);
        }
        Ok(Self(path))
    }

    /// Passed literally to the node binary; never shell-interpolated.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) async fn start_permit(&self) -> Result<StartPermit, FilesError> {
        let dir = DataDir::new(self.0.clone());
        tokio::task::spawn_blocking(move || {
            dir.prepare().map_err(FilesError::Store)?;
            let lock = StartLock::try_acquire(&dir)
                .map_err(FilesError::Store)?
                .ok_or(FilesError::Busy)?;
            Ok(StartPermit { _lock: lock })
        })
        .await
        .map_err(FilesError::Worker)?
    }

    pub(crate) async fn identity(&self) -> Result<Option<Identity>, FilesError> {
        let dir = DataDir::new(self.0.clone());
        tokio::task::spawn_blocking(move || {
            let Some(node) = dir.read_node_id().map_err(FilesError::Store)? else {
                return Ok(None);
            };
            let token = dir.read_token().map_err(FilesError::Store)?;
            let token: BearerToken = token.expose().parse().map_err(FilesError::Credential)?;
            Ok(Some(Identity {
                node,
                endpoint: Endpoint(dir.local_endpoint(node)),
                credential: LocalCredential::Token(token),
            }))
        })
        .await
        .map_err(FilesError::Worker)?
    }
}
