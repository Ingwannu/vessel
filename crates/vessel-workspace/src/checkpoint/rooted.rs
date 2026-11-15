//! Handle-relative no-follow traversal, identity and unsupported ACL refusal.

mod mount;
pub(super) mod mutation;
mod observation;

use super::Error;
use mount::MountId;
use rustix::fs::{CWD, Mode, OFlags};
use std::{
    fs::{File, Metadata},
    io,
    os::unix::fs::MetadataExt as _,
    path::Path,
};
use vessel_proto::checkpoint::{
    CheckpointError, CheckpointPath, FileMetadata, GroupId, OrdinaryMode, OwnerId,
};

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const REGULAR: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::NONBLOCK)
    .union(OFlags::CLOEXEC);
const FILE_TYPE_BITS: u32 = 0o170_000;
const ATTRIBUTE_NAMES_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    mode: u32,
    owner: u32,
    group: u32,
    links: u64,
}
impl Identity {
    pub(super) fn of(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.size(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            mode: metadata.mode(),
            owner: metadata.uid(),
            group: metadata.gid(),
            links: metadata.nlink(),
        }
    }
}
#[derive(Debug)]
pub(super) struct Root {
    file: File,
    mount: MountId,
    identity: Identity,
    physical: vessel_proto::checkpoint::PhysicalRoot,
    path: std::path::PathBuf,
}
impl Root {
    pub(super) fn open(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(CheckpointError::InvalidPath.into());
        }
        let file = File::from(
            rustix::fs::openat(CWD, path, DIRECTORY, Mode::empty()).map_err(io::Error::from)?,
        );
        let metadata = file.metadata()?;
        super::account::owned(&metadata)?;
        refuse_attributes(&file)?;
        let mount = MountId::of(&file)?;
        Ok(Self {
            identity: Identity::of(&metadata),
            physical: crate::physical::key(&metadata),
            file,
            mount,
            path: path.to_path_buf(),
        })
    }
    pub(super) fn revalidate(&self) -> Result<(), Error> {
        let current = Self::open(&self.path)?;
        if current.identity != self.identity || current.mount != self.mount {
            return Err(CheckpointError::CaptureUnstable.into());
        }
        Ok(())
    }
    pub(in crate::checkpoint) fn check_mount(&self, file: &File) -> Result<(), Error> {
        if MountId::of(file)? != self.mount {
            return Err(CheckpointError::UnsafeEntry.into());
        }
        Ok(())
    }
    pub(in crate::checkpoint) fn entry_identity(
        &self,
        path: &CheckpointPath,
    ) -> Result<Option<Identity>, Error> {
        Ok(self
            .file(path)?
            .map(|file| file.metadata())
            .transpose()?
            .as_ref()
            .map(Identity::of))
    }
    pub(super) fn file(&self, path: &CheckpointPath) -> Result<Option<File>, Error> {
        let mut parent = self.file.try_clone()?;
        let mut parts = path.relative().parts().peekable();
        while let Some(part) = parts.next() {
            let flags = if parts.peek().is_some() {
                DIRECTORY
            } else {
                REGULAR
            };
            let opened = rustix::fs::openat(&parent, part, flags, Mode::empty());
            let opened = match opened {
                Ok(opened) => File::from(opened),
                Err(rustix::io::Errno::NOENT) => return Ok(None),
                Err(rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) => {
                    return Err(CheckpointError::UnsafeEntry.into());
                }
                Err(error) => return Err(io::Error::from(error).into()),
            };
            let metadata = opened.metadata()?;
            self.check_mount(&opened)?;
            if metadata.dev() != self.identity.device {
                return Err(CheckpointError::UnsafeEntry.into());
            }
            if metadata.uid() != self.identity.owner {
                return Err(CheckpointError::UnsupportedMetadata.into());
            }
            refuse_attributes(&opened)?;
            if parts.peek().is_none() {
                if !metadata.is_file() || metadata.nlink() != 1 {
                    return Err(CheckpointError::UnsafeEntry.into());
                }
                return Ok(Some(opened));
            }
            // A nested repository is a separate scope, even if its files are listed.
            match rustix::fs::statat(&opened, ".git", rustix::fs::AtFlags::SYMLINK_NOFOLLOW) {
                Ok(_) => return Err(CheckpointError::UnsafeEntry.into()),
                Err(rustix::io::Errno::NOENT) => {}
                Err(error) => return Err(io::Error::from(error).into()),
            }
            parent = opened;
        }
        Err(CheckpointError::InvalidPath.into())
    }
}
pub(super) fn metadata(file: &File) -> Result<FileMetadata, Error> {
    let metadata = file.metadata()?;
    Ok(FileMetadata {
        owner: OwnerId::new(metadata.uid()),
        group: GroupId::new(metadata.gid()),
        mode: OrdinaryMode::new(metadata.mode() & !FILE_TYPE_BITS)?,
    })
}
/// Replacing a file cannot preserve ACL/xattr semantics; never silently drop them.
pub(super) fn refuse_attributes(file: &File) -> Result<(), Error> {
    let mut names = [0_u8; ATTRIBUTE_NAMES_BYTES];
    match rustix::fs::flistxattr(file, &mut names[..]) {
        Ok(0) | Err(rustix::io::Errno::NOTSUP) => Ok(()),
        Ok(_) | Err(rustix::io::Errno::RANGE) => Err(CheckpointError::UnsupportedMetadata.into()),
        Err(error) => Err(io::Error::from(error).into()),
    }
}
