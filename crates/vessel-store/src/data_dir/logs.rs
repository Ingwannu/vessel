//! The node's log files in its data directory: the one a background node
//! is started with, and the ones its reports read (ADR 0111: the files its
//! log may be in for `node.log`, and the space left on its file system for
//! the report's "Disk").

use std::path::PathBuf;

use super::{DataDir, LOG_DIR, NODE_LOG_FILE, NODE_LOG_ROTATE, StoreError, files};

/// The log `vessel init`'s launchd service writes (in [`LOG_DIR`]).
pub const SERVICE_LOG_FILE: &str = "vessel-service.log";

/// Bytes on a file system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    /// Free for this user.
    pub available: u64,
    /// In all.
    pub total: u64,
}

impl DataDir {
    /// The log file for a node started in the background, opened for
    /// appending (owner-only), after rotating a log that grew too large.
    /// Blocking.
    ///
    /// # Errors
    /// The log directory or file cannot be created or opened.
    pub fn open_node_log(&self) -> Result<(PathBuf, std::fs::File), StoreError> {
        let dir = self.log_dir();
        files::create_private_dir(&dir).map_err(StoreError::io(&dir))?;
        let path = dir.join(NODE_LOG_FILE);
        if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() > NODE_LOG_ROTATE) {
            let rotated = dir.join(format!("{NODE_LOG_FILE}.1"));
            std::fs::rename(&path, &rotated).map_err(StoreError::io(&rotated))?;
        }
        let file = files::open_append_private(&path).map_err(StoreError::io(&path))?;
        Ok((path, file))
    }

    /// The files this node's log may be: a background node's
    /// (`vesseld.log`) and a launchd service's, in that order.
    #[must_use]
    pub fn node_log_files(&self) -> [PathBuf; 2] {
        let dir = self.root.join(LOG_DIR);
        [dir.join(NODE_LOG_FILE), dir.join(SERVICE_LOG_FILE)]
    }

    /// The space on the file system holding this directory. Blocking.
    ///
    /// # Errors
    /// The file system cannot be asked, or this platform has no `statvfs`
    /// (`Unsupported`).
    pub fn space(&self) -> std::io::Result<Space> {
        space(&self.root)
    }
}

#[cfg(unix)]
fn space(path: &std::path::Path) -> std::io::Result<Space> {
    let stat = rustix::fs::statvfs(path)?;
    let bytes = |blocks: u64| blocks.saturating_mul(stat.f_frsize);
    Ok(Space {
        available: bytes(stat.f_bavail),
        total: bytes(stat.f_blocks),
    })
}

#[cfg(not(unix))]
fn space(_path: &std::path::Path) -> std::io::Result<Space> {
    Err(std::io::ErrorKind::Unsupported.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_files_are_in_the_log_folder_and_the_space_is_read() {
        let folder = tempfile::tempdir().unwrap();
        let dir = DataDir::new(folder.path().to_path_buf());
        let [node, service] = dir.node_log_files();
        assert_eq!(node, folder.path().join("logs").join("vesseld.log"));
        assert_eq!(service, folder.path().join("logs").join(SERVICE_LOG_FILE));
        if cfg!(unix) {
            let space = dir.space().unwrap();
            assert!(space.total > 0 && space.available <= space.total);
        }
    }
}
