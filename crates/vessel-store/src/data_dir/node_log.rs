//! Existing private node log policy shared by attach and desktop-managed launch.

use super::DataDir;
use crate::{StoreError, files};
use std::path::PathBuf;

const NODE_LOG_FILE: &str = "vesseld.log";
/// Existing attach policy: rotate above ten MiB before opening another writer.
const NODE_LOG_ROTATE: u64 = 10 * 1024 * 1024;

impl DataDir {
    /// The private node log location; observing this path grants no process authority.
    #[must_use]
    pub fn node_log_path(&self) -> PathBuf {
        self.log_dir().join(NODE_LOG_FILE)
    }
    /// The log file for a node started in the background, opened for
    /// appending (owner-only), after rotating a log that grew too large.
    /// Blocking.
    ///
    /// # Errors
    /// The log directory or file cannot be created or opened.
    pub fn open_node_log(&self) -> Result<(PathBuf, std::fs::File), StoreError> {
        let dir = self.log_dir();
        files::create_private_dir(&dir).map_err(StoreError::io(&dir))?;
        let path = self.node_log_path();
        if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() > NODE_LOG_ROTATE) {
            let rotated = dir.join(format!("{NODE_LOG_FILE}.1"));
            std::fs::rename(&path, &rotated).map_err(StoreError::io(&rotated))?;
        }
        let file = files::open_append_private(&path).map_err(StoreError::io(&path))?;
        Ok((path, file))
    }
}
