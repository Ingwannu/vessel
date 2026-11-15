//! [`NodeLock`]: one node per data directory, and who that node is.
//!
//! [Decision Log]
//! - Purpose: two nodes on one data directory would both open the event log
//!   and both claim the local socket. `vesseld attach` also needs to know
//!   whether a node is running, and the installer's cleanup needs to know
//!   which binary it runs (ADRs 0007 and 0008).
//! - Alternatives: a pid file (Zed): a pid says nothing once the process is
//!   gone and its number reused, so it can name an unrelated process; asking
//!   the socket (says "no node" while one is still starting).
//! - Choice: the node holds an exclusive OS lock on `vesseld.lock` for its
//!   whole life (std `File::try_lock`: `flock` on Unix, `LockFileEx` on
//!   Windows). The OS releases it when the process ends, however it ends,
//!   so the lock can never be stale. Who holds it is written to a separate
//!   file, `vesseld.owner`, because Windows forbids reading a locked range;
//!   that record is trusted only while the lock is held.
//! - Release: always an explicit unlock, not just closing the file. A
//!   `flock` belongs to the open file description, and a child process that
//!   another thread is starting shares it from `fork` until `exec` closes
//!   it. Closing only our descriptor in that moment left the lock held, so
//!   a node restarted at once (in the same process, or right after the old
//!   process exits) found its directory locked. Unlocking releases it for
//!   every copy.

mod query;

use std::fs::{File, TryLockError};
use std::io;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::data_dir::DataDir;
use crate::error::StoreError;
use crate::files;

/// The process that holds a data directory's node lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeOwner {
    /// Its process id.
    pub pid: u32,
    /// The binary it runs (cleanup keeps this one, ADR 0008).
    pub executable: PathBuf,
    /// Its version.
    pub version: String,
    /// When it took the lock.
    pub started_at: DateTime<Utc>,
}

pub use query::KernelLock;

/// Absence from procfs is not evidence that a directory lock is free.
/// PID namespaces and filesystem lock emulation can hide its writer.
#[derive(Debug)]
pub enum KernelProbe {
    /// The kernel exposes this lock's exclusive flock writer and file identity.
    Found(KernelLock),
    /// Callers must still use the actual OS lock to establish availability.
    NotVisible,
    /// No supported safe kernel query exists on this platform.
    Unsupported,
}

/// The node lock of one data directory, held until dropped (or until the
/// process ends).
#[derive(Debug)]
pub struct NodeLock {
    /// Kept open while held; unlocked on drop (see the module docs).
    file: File,
}

impl NodeLock {
    /// Take the lock and record `owner` as its holder.
    ///
    /// # Errors
    /// [`StoreError::Locked`] when another node holds it (with that node's
    /// record if readable); [`StoreError::Io`] when the files cannot be
    /// opened or written.
    pub fn acquire(dir: &DataDir, owner: &NodeOwner) -> Result<Self, StoreError> {
        let path = dir.lock_path();
        let file = files::open_private(&path).map_err(StoreError::io(&path))?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(StoreError::Locked {
                    path,
                    holder: read_owner(dir).map(Box::new),
                });
            }
            Err(TryLockError::Error(source)) => return Err(StoreError::Io { path, source }),
        }
        let owner_path = dir.owner_path();
        let record = serde_json::to_vec(owner).map_err(StoreError::Encode)?;
        files::replace(&owner_path, &record).map_err(StoreError::io(&owner_path))?;
        Ok(Self { file })
    }

    /// Who holds the lock right now; `None` when no node does (any owner
    /// record left behind is then stale and ignored).
    ///
    /// # Errors
    /// The lock file cannot be opened or tested.
    pub fn holder(dir: &DataDir) -> Result<Option<NodeOwner>, StoreError> {
        let path = dir.lock_path();
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(StoreError::Io { path, source }),
        };
        match file.try_lock() {
            // Nobody holds it; give it back at once.
            Ok(()) => {
                release(&file);
                Ok(None)
            }
            Err(TryLockError::WouldBlock) => Ok(read_owner(dir)),
            Err(TryLockError::Error(source)) => Err(StoreError::Io { path, source }),
        }
    }

    /// Observe the selected lock without acquiring it or trusting its record.
    /// `NotVisible` does not mean free: the OS lock remains the availability proof.
    ///
    /// # Errors
    /// Lock metadata or bounded kernel/process observations cannot be read,
    /// decoded or confirmed for the same candidate.
    #[cfg(target_os = "linux")]
    pub fn kernel_holder(dir: &DataDir) -> Result<KernelProbe, StoreError> {
        let path = dir.lock_path();
        Ok(query::observe(&path)
            .map_err(StoreError::io(&path))?
            .map_or(KernelProbe::NotVisible, KernelProbe::Found))
    }

    /// Other systems retain the OS availability probe without inventing a PID.
    ///
    /// # Errors
    /// Never: this explicitly reports the unsupported kernel edge.
    #[cfg(not(target_os = "linux"))]
    pub const fn kernel_holder(_dir: &DataDir) -> Result<KernelProbe, StoreError> {
        Ok(KernelProbe::Unsupported)
    }
}

/// Held by a `vesseld attach` while it starts a node, so two of them
/// racing to start one do not both try: the second waits, then finds the
/// node the first one started.
#[derive(Debug)]
pub struct StartLock {
    /// Kept open while held; unlocked on drop (see the module docs).
    file: File,
}

impl StartLock {
    /// Wait for the start lock (blocking: call it off the async threads).
    ///
    /// # Errors
    /// The lock file cannot be opened or locked.
    pub fn acquire(dir: &DataDir) -> Result<Self, StoreError> {
        let path = dir.start_lock_path();
        let file = files::open_private(&path).map_err(StoreError::io(&path))?;
        file.lock().map_err(StoreError::io(&path))?;
        Ok(Self { file })
    }
}

impl Drop for NodeLock {
    fn drop(&mut self) {
        release(&self.file);
    }
}

impl Drop for StartLock {
    fn drop(&mut self) {
        release(&self.file);
    }
}

/// Unlock `file` for every descriptor that shares it (see the module docs).
/// If that fails, closing the file still releases it, only later.
fn release(file: &File) {
    if let Err(error) = file.unlock() {
        tracing::debug!(%error, "cannot unlock; closing the file releases the lock");
    }
}

/// The owner record, if readable. Only meaningful while the lock is held.
fn read_owner(dir: &DataDir) -> Option<NodeOwner> {
    let path = dir.owner_path();
    let text = std::fs::read(&path)
        .inspect_err(
            |error| tracing::debug!(path = %path.display(), %error, "no node owner record"),
        )
        .ok()?;
    serde_json::from_slice(&text)
        .inspect_err(
            |error| tracing::warn!(path = %path.display(), %error, "unreadable node owner record"),
        )
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> NodeOwner {
        NodeOwner {
            pid: std::process::id(),
            executable: PathBuf::from("vesseld"),
            version: "0.0.0".to_owned(),
            started_at: Utc::now(),
        }
    }

    /// A second descriptor of the lock file (what a child process has from
    /// `fork` until `exec`) does not keep a dropped lock held.
    #[test]
    fn a_dropped_lock_is_free_even_while_a_copy_of_its_file_is_open() {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let lock = NodeLock::acquire(&dir, &owner()).unwrap();
        let copy = lock.file.try_clone().unwrap();
        drop(lock);
        assert_eq!(NodeLock::holder(&dir).unwrap(), None);
        drop(NodeLock::acquire(&dir, &owner()).unwrap());
        drop(copy);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn kernel_observation_preserves_the_held_record_and_reports_the_writer() {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let record = owner();
        let lock = NodeLock::acquire(&dir, &record).unwrap();
        let before = std::fs::read(dir.owner_path()).unwrap();
        let KernelProbe::Found(_) = NodeLock::kernel_holder(&dir).unwrap() else {
            panic!("no kernel writer");
        };
        assert_eq!(before, std::fs::read(dir.owner_path()).unwrap());
        std::fs::write(dir.owner_path(), b"damaged").unwrap();
        assert!(matches!(
            NodeLock::kernel_holder(&dir).unwrap(),
            KernelProbe::Found(_)
        ));
        assert!(NodeLock::holder(&dir).unwrap().is_none());
        drop(lock);
        assert!(matches!(
            NodeLock::kernel_holder(&dir).unwrap(),
            KernelProbe::NotVisible
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_missing_lock_is_not_visible_and_the_query_creates_no_file() {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        assert!(matches!(
            NodeLock::kernel_holder(&dir).unwrap(),
            KernelProbe::NotVisible
        ));
        assert!(!dir.lock_path().exists());
        assert!(!dir.owner_path().exists());
    }
}
