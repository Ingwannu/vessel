//! Unix: every folder opened relative to the one before, never through a
//! link (`O_NOFOLLOW | O_DIRECTORY`), the file checked with `fstat`.

use std::fs::File;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

use rustix::fs::{AtFlags, CWD, Mode, OFlags};
use rustix::io::Errno;

use super::FileIdentity;

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// An open folder; everything below it is named relative to it.
#[derive(Debug)]
pub(super) struct Dir(OwnedFd);

impl Dir {
    /// `path` itself must not be a link; the folders above it are the
    /// data directory's, which the node made.
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        Ok(Self(rustix::fs::openat(
            CWD,
            path,
            DIRECTORY,
            Mode::empty(),
        )?))
    }

    pub(super) fn open_dir(&self, name: &str) -> io::Result<Self> {
        Ok(Self(rustix::fs::openat(
            &self.0,
            name,
            DIRECTORY,
            Mode::empty(),
        )?))
    }

    /// The folder `name`, made owner-only if it is not there.
    pub(super) fn ensure_dir(&self, name: &str) -> io::Result<Self> {
        match rustix::fs::mkdirat(&self.0, name, Mode::RWXU) {
            Ok(()) | Err(Errno::EXIST) => self.open_dir(name),
            Err(error) => Err(error.into()),
        }
    }

    /// A new owner-only folder `name`; an error if anything is there.
    pub(super) fn create_dir(&self, name: &str) -> io::Result<Self> {
        rustix::fs::mkdirat(&self.0, name, Mode::RWXU)?;
        self.open_dir(name)
    }

    /// The regular file `name`; an error for a link or anything else.
    pub(super) fn open_file(&self, name: &str) -> io::Result<File> {
        let flags = OFlags::RDONLY
            .union(OFlags::NOFOLLOW)
            .union(OFlags::NONBLOCK)
            .union(OFlags::CLOEXEC);
        let file = File::from(rustix::fs::openat(&self.0, name, flags, Mode::empty())?);
        if !file.metadata()?.file_type().is_file() {
            return Err(Errno::LOOP.into());
        }
        Ok(file)
    }

    /// Move the file at `from` here as `name`.
    pub(super) fn rename_into(&self, from: &Path, name: &str) -> io::Result<()> {
        Ok(rustix::fs::renameat(CWD, from, &self.0, name)?)
    }

    pub(super) fn remove_file(&self, name: &str) -> io::Result<()> {
        Ok(rustix::fs::unlinkat(&self.0, name, AtFlags::empty())?)
    }

    pub(super) fn remove_dir(&self, name: &str) -> io::Result<()> {
        Ok(rustix::fs::unlinkat(&self.0, name, AtFlags::REMOVEDIR)?)
    }
}

/// A link (`ELOOP`) or a file where a folder was expected (`ENOTDIR`).
pub(super) fn is_link_refusal(error: &io::Error) -> bool {
    let code = Errno::from_io_error(error);
    code == Some(Errno::LOOP) || code == Some(Errno::NOTDIR)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "one signature on every platform: Windows has no identity"
)]
pub(super) fn identity(metadata: &std::fs::Metadata) -> Option<FileIdentity> {
    Some(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        size: metadata.size(),
        modified: (metadata.mtime(), metadata.mtime_nsec()),
        changed: (metadata.ctime(), metadata.ctime_nsec()),
    })
}
