//! Private metadata is validated on opened directories, never repaired on reuse.
use crate::checkpoint::{Error, account, rooted::MountId};
use rustix::fs::{AtFlags, CWD, Mode, OFlags};
use std::{
    ffi::CStr,
    fs::File,
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
};
use vessel_proto::checkpoint::CheckpointError;

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

#[derive(Debug)]
pub(super) struct Directory {
    pub(super) file: File,
    pub(super) path: PathBuf,
    pub(super) mount: MountId,
}
impl Directory {
    pub(super) fn open(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(CheckpointError::InvalidPath.into());
        }
        let file = File::from(
            rustix::fs::openat(CWD, path, DIRECTORY, Mode::empty())
                .map_err(std::io::Error::from)?,
        );
        account::private_directory(&file)?;
        let mount = MountId::of(&file)?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            mount,
        })
    }
    pub(super) fn child(&self, name: &CStr) -> Result<Self, Error> {
        account::private_directory(&self.file)?;
        let file = File::from(
            rustix::fs::openat(&self.file, name, DIRECTORY, Mode::empty())
                .map_err(std::io::Error::from)?,
        );
        account::private_directory(&file)?;
        let mount = MountId::of(&file)?;
        if mount != self.mount {
            return Err(CheckpointError::UnsafeEntry.into());
        }
        Ok(Self {
            file,
            path: self.path.join(std::ffi::OsStr::from_bytes(name.to_bytes())),
            mount,
        })
    }
    pub(super) fn create(&self, name: &CStr) -> Result<Self, Error> {
        self.mkdir(name)?;
        self.child(name)
    }
    pub(super) fn ensure(&self, name: &CStr) -> Result<Self, Error> {
        match self.mkdir(name) {
            Ok(()) => self.child(name),
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.child(name)
            }
            Err(error) => Err(error),
        }
    }
    fn mkdir(&self, name: &CStr) -> Result<(), Error> {
        account::private_directory(&self.file)?;
        rustix::fs::mkdirat(&self.file, name, Mode::RWXU).map_err(std::io::Error::from)?;
        // mkdir applies umask but returns no descriptor. No-follow chmod applies
        // only to this newly created child, never an existing access policy.
        rustix::fs::chmodat(&self.file, name, Mode::RWXU, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(std::io::Error::from)?;
        let child = self.child(name)?;
        child.file.sync_all()?;
        self.file.sync_all()?;
        Ok(())
    }
}
