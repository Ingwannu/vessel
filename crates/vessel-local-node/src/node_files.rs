//! The sole Store facade consumer; never opens `SQLite` or trusts an owner PID.

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
    /// Explicit bounded reading of the managed log failed.
    #[error("local node log file could not be read")]
    Log(#[source] std::io::Error),
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

    /// Existing private Store log, shared with the node binary and attach.
    #[must_use]
    pub fn log_path(&self) -> PathBuf {
        DataDir::new(self.0.clone()).node_log_path()
    }

    /// Read only the bounded final bytes and lines, including after app restart.
    ///
    /// # Errors
    /// File access, range conversion or blocking worker failure.
    pub async fn log_tail(&self) -> Result<crate::LogText, FilesError> {
        use std::io::{Read as _, Seek as _};
        let path = self.log_path();
        tokio::task::spawn_blocking(move || {
            let mut file = std::fs::File::open(path).map_err(FilesError::Log)?;
            let length = file.metadata().map_err(FilesError::Log)?.len();
            let limit = u64::try_from(crate::logs::MAX_LOG_BYTES)
                .map_err(|error| FilesError::Log(std::io::Error::other(error)))?;
            let _position = file
                .seek(std::io::SeekFrom::Start(length.saturating_sub(limit)))
                .map_err(FilesError::Log)?;
            let mut bytes = Vec::with_capacity(crate::logs::MAX_LOG_BYTES);
            let _read = file
                .take(limit)
                .read_to_end(&mut bytes)
                .map_err(FilesError::Log)?;
            Ok(crate::logs::tail(&bytes))
        })
        .await
        .map_err(FilesError::Worker)?
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
        tokio::task::spawn_blocking(move || read_identity(&dir))
            .await
            .map_err(FilesError::Worker)?
    }
}

fn read_identity(dir: &DataDir) -> Result<Option<Identity>, FilesError> {
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
}

/// A causal test deliberately holds the only blocking worker at a barrier.
/// Read its isolated fixture through the SAME facade without queueing behind
/// that worker; no public API, extra parser, database or Store edge is added.
#[cfg(test)]
pub(crate) fn fixture_identity(home: &NodeHome) -> Result<Option<Identity>, FilesError> {
    read_identity(&DataDir::new(home.0.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[tokio::test]
    async fn retained_tail_has_the_existing_private_path_and_last_hundred_lines() {
        let root = tempfile::tempdir().unwrap();
        let dir = DataDir::new(root.path().join("node"));
        dir.prepare().unwrap();
        let (path, mut log) = dir.open_node_log().unwrap();
        for line in 0..200 {
            writeln!(log, "line-{line:03} {}", "x".repeat(1024)).unwrap();
        }
        drop(log);
        let home = NodeHome::new(dir.root().to_owned()).unwrap();
        assert_eq!(home.log_path(), path);
        let tail = home.log_tail().await.unwrap();
        assert_eq!(tail.as_str().lines().count(), 100);
        assert!(tail.as_str().starts_with("line-100 "));
        assert!(
            tail.as_str()
                .lines()
                .last()
                .unwrap()
                .starts_with("line-199 ")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
