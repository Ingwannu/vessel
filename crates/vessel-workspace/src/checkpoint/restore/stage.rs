//! Restore staging is private and complete before any project file is replaced.
use crate::checkpoint::{
    Error,
    content::{self, Budget},
    inspect,
    rooted::{Identity, metadata, refuse_attributes},
};
use rustix::fs::{CWD, Gid, Mode, OFlags};
use std::{fs::File, os::unix::fs::MetadataExt as _, path::Path};
use vessel_proto::{
    BlobHash,
    checkpoint::{CheckpointError, CheckpointFileBytes, FileMetadata, PathCount},
};

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const CREATE: OFlags = OFlags::WRONLY
    .union(OFlags::CREATE)
    .union(OFlags::EXCL)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const PRIVATE_MODE: u32 = 0o600;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::checkpoint) enum SlotKind {
    Desired,
    Deleted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::checkpoint) struct Slot {
    index: PathCount,
    kind: SlotKind,
}
impl Slot {
    pub(in crate::checkpoint) const fn new(index: PathCount, kind: SlotKind) -> Self {
        Self { index, kind }
    }
    pub(in crate::checkpoint) fn text(self) -> String {
        let prefix = match self.kind {
            SlotKind::Desired => "desired",
            SlotKind::Deleted => "deleted",
        };
        format!("{prefix}-{}", self.index.get())
    }
}
#[derive(Debug)]
pub(in crate::checkpoint) struct Stage {
    pub(in crate::checkpoint) folder: File,
}
impl Stage {
    pub(in crate::checkpoint) fn open(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(CheckpointError::InvalidPath.into());
        }
        let folder = File::from(
            rustix::fs::openat(CWD, path, DIRECTORY, Mode::empty())
                .map_err(std::io::Error::from)?,
        );
        crate::checkpoint::account::private_directory(&folder)?;
        Ok(Self { folder })
    }
    pub(in crate::checkpoint) fn prepare(
        &self,
        slot: Slot,
        source: &mut File,
        digest: BlobHash,
        bytes: CheckpointFileBytes,
        desired: FileMetadata,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<(), Error> {
        let mut output = File::from(
            rustix::fs::openat(
                &self.folder,
                slot.text(),
                CREATE,
                Mode::from_raw_mode(PRIVATE_MODE),
            )
            .map_err(std::io::Error::from)?,
        );
        let (found, _bytes) =
            content::transfer(source, &mut output, Budget::Exact(bytes), cancellation)?;
        if found != digest {
            return Err(CheckpointError::CaptureUnstable.into());
        }
        let found = output.metadata()?;
        if found.uid() != desired.owner.get() {
            return Err(CheckpointError::UnsupportedMetadata.into());
        }
        if found.gid() != desired.group.get() {
            rustix::fs::fchown(&output, None, Some(Gid::from_raw(desired.group.get())))
                .map_err(std::io::Error::from)?;
        }
        rustix::fs::fchmod(&output, Mode::from_raw_mode(desired.mode.get()))
            .map_err(std::io::Error::from)?;
        if metadata(&output)? != desired {
            return Err(CheckpointError::UnsupportedMetadata.into());
        }
        output.sync_all()?;
        self.folder.sync_all()?;
        Ok(())
    }
    pub(in crate::checkpoint) fn inspect(
        &self,
        slot: Slot,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<vessel_proto::checkpoint::FileState, Error> {
        inspect::file(&mut self.file(slot)?, cancellation)
    }
    pub(in crate::checkpoint) fn identity(&self, slot: Slot) -> Result<Identity, Error> {
        Ok(Identity::of(&self.file(slot)?.metadata()?))
    }
    fn file(&self, slot: Slot) -> Result<File, Error> {
        let flags = OFlags::RDONLY
            .union(OFlags::NOFOLLOW)
            .union(OFlags::NONBLOCK)
            .union(OFlags::CLOEXEC);
        let source = File::from(
            rustix::fs::openat(&self.folder, slot.text(), flags, Mode::empty())
                .map_err(std::io::Error::from)?,
        );
        let found = source.metadata()?;
        if !found.is_file() || found.nlink() != 1 || found.uid() != self.folder.metadata()?.uid() {
            return Err(CheckpointError::UnsafeEntry.into());
        }
        refuse_attributes(&source)?;
        Ok(source)
    }
    #[cfg(test)]
    pub(in crate::checkpoint) fn remove(&self, slot: Slot) -> Result<(), Error> {
        rustix::fs::unlinkat(&self.folder, slot.text(), rustix::fs::AtFlags::empty())
            .map_err(std::io::Error::from)?;
        self.folder.sync_all()?;
        Ok(())
    }
}
