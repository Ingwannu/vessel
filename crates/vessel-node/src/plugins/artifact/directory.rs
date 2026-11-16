//! Staged descriptor-relative reads, not candidate storage or root admission.
//!
//! Why (ADR0079): a retained directory must not silently reopen its old pathname.
//! This checks only the selected directory and descendant opens. Ancestry,
//! acquisition ownership, complete observation and installation remain absent.
use super::{PackagePath, file, tree};
use nix::{
    errno::Errno,
    fcntl::{AtFlags, OFlag, OpenHow, ResolveFlag, openat2},
    sys::stat::{SFlag, fstatat},
};
use std::{
    fs::{File, OpenOptions},
    os::{
        fd::OwnedFd,
        unix::fs::{MetadataExt as _, OpenOptionsExt as _},
    },
    path::Path,
};

#[derive(Debug, thiserror::Error)]
enum Failure {
    #[error("plugin descriptor resolution is unsupported")]
    Unsupported(#[source] Errno),
    #[error("plugin descriptor operation failed")]
    System(#[source] Errno),
    #[error("plugin descriptor file policy refused observation")]
    File(#[from] file::Failure),
    #[error("plugin descriptor directory policy refused observation")]
    Directory(#[from] tree::Failure),
}

/// A checked existing directory descriptor, NOT an authenticated candidate/root.
/// Opening this path does not verify its ancestors or grant deletion authority.
struct DirectoryProbe {
    root: File,
    device: u64,
}
impl DirectoryProbe {
    fn open(path: &Path) -> Result<Self, Failure> {
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
            .open(path)
            .map_err(file::io_failure)?;
        let metadata = root.metadata().map_err(file::io_failure)?;
        Self::from_file(root, metadata.dev())
    }

    fn from_file(root: File, device: u64) -> Result<Self, Failure> {
        let metadata = root.metadata().map_err(file::io_failure)?;
        tree::directory(&metadata, device)?;
        Ok(Self { root, device })
    }

    fn open_directory(&self, path: &PackagePath) -> Result<Self, Failure> {
        self.checked_directory(path.as_str())
    }

    fn checked_directory(&self, path: &str) -> Result<Self, Failure> {
        let mut current = Self::from_file(
            self.root.try_clone().map_err(file::io_failure)?,
            self.device,
        )?;
        for component in path.split('/') {
            let next = resolved(
                &current.root,
                component,
                OFlag::O_RDONLY | OFlag::O_DIRECTORY,
            )?;
            current = Self::from_file(next.into(), self.device)?;
        }
        Ok(current)
    }

    fn open_file(&self, path: &PackagePath) -> Result<File, Failure> {
        self.open_file_with_gate(path, || {})
    }

    fn open_file_with_gate(
        &self,
        path: &PackagePath,
        before_open: impl FnOnce(),
    ) -> Result<File, Failure> {
        let (parent, name) = match path.as_str().rsplit_once('/') {
            Some((parent, name)) => (self.checked_directory(parent)?.root, name),
            None => (
                Self::from_file(
                    self.root.try_clone().map_err(file::io_failure)?,
                    self.device,
                )?
                .root,
                path.as_str(),
            ),
        };
        // Resolve the parent first so this no-follow metadata call cannot walk
        // an unverified ancestor link. Refuse special kinds before opening.
        let before = fstatat(&parent, name, AtFlags::AT_SYMLINK_NOFOLLOW).map_err(system)?;
        if SFlag::from_bits_truncate(before.st_mode) & SFlag::S_IFMT != SFlag::S_IFREG {
            return Err(file::Failure::Policy.into());
        }
        before_open();
        let fd = resolved(&parent, name, OFlag::O_RDONLY | OFlag::O_NONBLOCK)?;
        let opened = File::from(fd);
        let after = opened.metadata().map_err(file::io_failure)?;
        file::regular(&after, self.device)?;
        if after.dev() != before.st_dev || after.ino() != before.st_ino {
            return Err(file::Failure::Changed.into());
        }
        Ok(opened)
    }
}

fn resolved(parent: &File, path: &str, flags: OFlag) -> Result<OwnedFd, Failure> {
    let how = OpenHow::new()
        .flags(flags | OFlag::O_CLOEXEC | OFlag::O_NOFOLLOW)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH
                | ResolveFlag::RESOLVE_NO_XDEV
                | ResolveFlag::RESOLVE_NO_MAGICLINKS
                | ResolveFlag::RESOLVE_NO_SYMLINKS,
        );
    // No retry with weaker flags or absolute/proc-path reopening.
    openat2(parent, path, how).map_err(system)
}

fn system(cause: Errno) -> Failure {
    if matches!(cause, Errno::ENOSYS | Errno::EINVAL | Errno::E2BIG) {
        Failure::Unsupported(cause)
    } else {
        Failure::System(cause)
    }
}

#[cfg(test)]
mod tests;
