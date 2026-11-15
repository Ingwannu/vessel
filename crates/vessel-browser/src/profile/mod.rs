//! Linux private profile initialization, staged below the future supervised launcher.
//!
//! Caller paths are trusted node storage configuration, never browser/tool input.
//! Run this synchronous preparation through the finite `Preparation` owner. This
//! module does not launch/qualify a runtime or delete a profile on caller drop.
//! The supervisor must hold this lock until actual child cleanup completes.
use std::fs::{self, DirBuilder, File, OpenOptions, TryLockError};
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};

/// Chrome149 adds its own temporary `SingletonSocket` suffix. Leave margin below
/// Linux's Unix-domain socket path limit rather than aborting inside Chromium.
const MAX_RUNTIME_TEMP_PATH_BYTES: usize = 48;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;
const OTHER_ACCESS: u32 = 0o077;
const PARENT_WRITE: u32 = 0o022;
const STICKY: u32 = 0o1000;
const ROOT_UID: u32 = 0;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProfileError {
    #[error("browser profile filesystem operation failed")]
    Io(#[source] std::io::Error),
    #[error("browser profile paths must be absolute and free of symlinks")]
    Path,
    #[error("browser profile permissions or ownership are not private")]
    Permissions,
    #[error("browser runtime temporary path is too long")]
    TemporaryPath,
    #[error("browser profile is already owned by another process")]
    Busy,
}
/// No path or credential-bearing filename is rendered by Debug.
pub(crate) struct Profile {
    root: PathBuf,
    temp: PathBuf,
    lock: File,
}
impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserProfile").finish_non_exhaustive()
    }
}
impl Profile {
    /// Existing unsafe directories/files are refused, never chmod-repaired.
    pub(crate) fn prepare(root: &Path, temp: &Path) -> Result<Self, ProfileError> {
        if temp.as_os_str().as_encoded_bytes().len() > MAX_RUNTIME_TEMP_PATH_BYTES {
            return Err(ProfileError::TemporaryPath);
        }
        private_directory(root)?;
        private_directory(temp)?;
        for relative in ["home", "config", "cache", "cache/fontconfig"] {
            private_directory(&root.join(relative))?;
        }
        let path = root.join(".vessel-browser.lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(PRIVATE_FILE_MODE)
            .custom_flags(nix::libc::O_NOFOLLOW)
            .open(path)
            .map_err(ProfileError::Io)?;
        let metadata = lock.metadata().map_err(ProfileError::Io)?;
        if !metadata.is_file()
            || metadata.uid() != nix::unistd::geteuid().as_raw()
            || metadata.mode() & OTHER_ACCESS != 0
            || metadata.nlink() != 1
        {
            return Err(ProfileError::Permissions);
        }
        lock.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => ProfileError::Busy,
            TryLockError::Error(source) => ProfileError::Io(source),
        })?;
        Ok(Self {
            root: root.to_owned(),
            temp: temp.to_owned(),
            lock,
        })
    }
    /// Only the process supervisor calls this after actual child cleanup. As
    /// `NodeLock` does, explicitly unlock copies held transiently across fork.
    pub(crate) fn release(self) -> Result<(), ProfileError> {
        self.lock.unlock().map_err(ProfileError::Io)
    }
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn home(&self) -> PathBuf {
        self.root.join("home")
    }
    pub(crate) fn config(&self) -> PathBuf {
        self.root.join("config")
    }
    pub(crate) fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }
    pub(crate) fn temporary(&self) -> &Path {
        &self.temp
    }
}

fn private_directory(path: &Path) -> Result<(), ProfileError> {
    if !path.is_absolute() {
        return Err(ProfileError::Path);
    }
    let parent = path.parent().ok_or(ProfileError::Path)?;
    protected_ancestors(parent)?;
    match DirBuilder::new().mode(PRIVATE_DIRECTORY_MODE).create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(source) => return Err(ProfileError::Io(source)),
    }
    let metadata = fs::symlink_metadata(path).map_err(ProfileError::Io)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ProfileError::Path);
    }
    if metadata.uid() != nix::unistd::geteuid().as_raw() || metadata.mode() & OTHER_ACCESS != 0 {
        return Err(ProfileError::Permissions);
    }
    // An ancestor containing '..' or a link must not be normalized into a
    // different credential root after its ownership check.
    if fs::canonicalize(path).map_err(ProfileError::Io)? != path {
        return Err(ProfileError::Path);
    }
    Ok(())
}
fn protected_ancestors(path: &Path) -> Result<(), ProfileError> {
    let uid = nix::unistd::geteuid().as_raw();
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(ProfileError::Io)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ProfileError::Path);
        }
        if metadata.uid() != uid && metadata.uid() != ROOT_UID {
            return Err(ProfileError::Permissions);
        }
        let sticky_root = metadata.uid() == ROOT_UID && metadata.mode() & STICKY != 0;
        if metadata.mode() & PARENT_WRITE != 0 && !sticky_root {
            return Err(ProfileError::Permissions);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
