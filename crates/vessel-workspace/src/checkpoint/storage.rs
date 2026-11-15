//! Typed private namespaces; the node reserves quota before calling creation.
//! Every operation runs on its retained admitted blocking worker, never at Drop.
mod cleanup;
mod directory;
use super::{Archive, Error, archive};
use directory::Directory;
use std::{ffi::CString, path::Path};
use vessel_proto::checkpoint::{CaptureKey, CheckpointError, RestoreId, RestorePlanId};

/// A bounded private cleanup pass; only Complete permits a quota acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanup {
    /// More private entries remain; keep the durable charge and retry a pass.
    Pending,
    /// Namespace absence and its parent directory are durably synced.
    Complete,
}

/// Opened service-owned checkpoint storage below the prepared node data folder.
#[derive(Debug)]
pub struct Storage {
    folder: Directory,
}
/// One capture's private archive, previews and restore stages; no client paths.
#[derive(Debug)]
pub struct CaptureStorage {
    folder: Directory,
}
/// A validated stage location held open while the node prepares/mutates files.
#[derive(Debug)]
pub struct StageDirectory {
    folder: Directory,
}

fn name(id: impl std::fmt::Display) -> Result<CString, Error> {
    CString::new(id.to_string()).map_err(|_invalid| CheckpointError::InvalidPath.into())
}
fn archive_from(folder: Directory) -> Result<Archive, Error> {
    Ok(Archive(archive::Archive::from_folder(
        folder.file,
        folder.path,
    )?))
}
impl Storage {
    /// Open/prepare checkpoints below an existing private service data folder.
    /// Existing directory ownership, access bits and attributes are never changed.
    /// # Errors
    /// Unsafe ownership/mode/mount, symlink, unsupported syscall or filesystem I/O.
    pub fn open(data: &Path) -> Result<Self, Error> {
        Ok(Self {
            folder: Directory::open(data)?.ensure(c"checkpoints")?,
        })
    }
    /// Create a non-reused capture namespace after durable fixed-escrow admission.
    /// # Errors
    /// Existing identity, unsafe directory metadata or filesystem I/O failure.
    pub fn create_capture(&self, key: CaptureKey) -> Result<CaptureStorage, Error> {
        let worktree = self.folder.ensure(&name(key.worktree)?)?;
        let capture = worktree.create(&name(key.capture)?)?;
        for child in [c"blobs", c"previews", c"restores"] {
            let _created = capture.create(child)?;
        }
        Ok(CaptureStorage { folder: capture })
    }
    /// Reopen the exact validated capture namespace; no directory is created.
    /// # Errors
    /// Missing identity, unsafe metadata/mount or filesystem I/O failure.
    pub fn open_capture(&self, key: CaptureKey) -> Result<CaptureStorage, Error> {
        Ok(CaptureStorage {
            folder: self
                .folder
                .child(&name(key.worktree)?)?
                .child(&name(key.capture)?)?,
        })
    }
}
impl CaptureStorage {
    /// Open this capture's immutable blobs without reopening through a path.
    /// # Errors
    /// Unsafe private metadata/mount or filesystem I/O failure.
    pub fn archive(&self) -> Result<Archive, Error> {
        archive_from(self.folder.child(c"blobs")?)
    }
    /// Create the preview's current-image archive after private quota reservation.
    /// # Errors
    /// Reused identity, unsafe metadata/mount or filesystem I/O failure.
    pub fn create_preview(&self, plan: RestorePlanId) -> Result<Archive, Error> {
        archive_from(self.folder.child(c"previews")?.create(&name(plan)?)?)
    }
    /// Reopen the exact preview archive pinned by the durable plan.
    /// # Errors
    /// Missing identity, unsafe metadata/mount or filesystem I/O failure.
    pub fn open_preview(&self, plan: RestorePlanId) -> Result<Archive, Error> {
        archive_from(self.folder.child(c"previews")?.child(&name(plan)?)?)
    }
    /// Create the restore's same-filesystem stage after atomic restore admission.
    /// # Errors
    /// Reused identity, unsafe metadata/mount or filesystem I/O failure.
    pub fn create_restore(&self, restore: RestoreId) -> Result<StageDirectory, Error> {
        Ok(StageDirectory {
            folder: self
                .folder
                .child(c"restores")?
                .create(&name(restore)?)?
                .create(c"stage")?,
        })
    }
    /// Reopen the durable restore stage for guarded recovery.
    /// # Errors
    /// Missing identity, unsafe metadata/mount or filesystem I/O failure.
    pub fn open_restore(&self, restore: RestoreId) -> Result<StageDirectory, Error> {
        Ok(StageDirectory {
            folder: self
                .folder
                .child(c"restores")?
                .child(&name(restore)?)?
                .child(c"stage")?,
        })
    }
}
impl StageDirectory {
    /// Private location for the existing restore API; the descriptor stays owned.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.folder.path
    }
}

#[cfg(test)]
mod tests;
