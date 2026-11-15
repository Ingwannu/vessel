//! Directory-relative Unix opens; never follow the final link or block on a FIFO.
//! The configured directory is the caller's trusted anchor, not a node input.

use std::fs::File;
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

use rustix::fs::{CWD, Mode, OFlags};
use rustix::io::Errno;

use super::{Error, Name, Purpose, Text, read_limited};

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const INPUT: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::NONBLOCK)
    .union(OFlags::CLOEXEC);

/// An already opened directory; replacing its pathname cannot redirect a read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectoryOwner(u32);

/// Ownership is relative to the trusted opened anchor, not a claimed remote
/// user or an invented current-process uid.
pub(super) struct Directory {
    file: File,
    owner: DirectoryOwner,
}
impl Directory {
    /// Open the trusted anchor once, refusing a final directory symlink.
    pub(super) fn open(path: &Path) -> Result<Self, Error> {
        let fd = rustix::fs::openat(CWD, path, DIRECTORY, Mode::empty()).map_err(open_error)?;
        let file = File::from(fd);
        let metadata = file.metadata().map_err(Error::Io)?;
        // Another user must not be able to replace entries in our trusted
        // anchor. Preserve existing modes; refuse instead of chmod.
        if metadata.mode() & 0o022 != 0 {
            return Err(Error::UnsafeTarget);
        }
        let owner = DirectoryOwner(metadata.uid());
        Ok(Self { file, owner })
    }

    /// Read only an ordinary file belonging to that anchor, with its format's cap.
    pub(super) fn read(&self, name: Name<'_>, purpose: Purpose) -> Result<Option<Text>, Error> {
        let fd = match rustix::fs::openat(&self.file, name.0, INPUT, Mode::empty()) {
            Ok(fd) => fd,
            Err(Errno::NOENT) => return Ok(None),
            Err(error) => return Err(open_error(error)),
        };
        let mut file = File::from(fd);
        let metadata = file.metadata().map_err(Error::Io)?;
        if !metadata.is_file() || DirectoryOwner(metadata.uid()) != self.owner {
            return Err(Error::UnsafeTarget);
        }
        // Books can be shared read-only. Credentials contain bearer material
        // and must not be exposed even read-only, matching store's push key.
        let forbidden = match purpose {
            Purpose::Book => 0o022,
            Purpose::Credential => 0o077,
        };
        if metadata.mode() & forbidden != 0 || metadata.mode() & 0o400 == 0 {
            return Err(Error::UnsafeTarget);
        }
        let limit = purpose.limit();
        if metadata.len() > u64::from(limit.0) {
            return Err(Error::TooLarge { limit });
        }
        read_limited(&mut file, limit).map(Some)
    }
}

fn open_error(error: Errno) -> Error {
    match error {
        Errno::LOOP | Errno::NOTDIR => Error::UnsafeTarget,
        other => Error::Io(other.into()),
    }
}
