//! Handle-relative project mutations stay inside the recorded root.
use super::{DIRECTORY, Root, refuse_attributes};
use crate::checkpoint::{
    Error,
    restore::stage::{Slot, Stage},
};
use rustix::fs::{Mode, RenameFlags};
use std::{fs::File, io};
use vessel_proto::checkpoint::{CheckpointError, CheckpointPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::checkpoint) enum ParentPolicy {
    Existing,
    CreateMissing,
}
#[derive(Debug)]
struct Component(String);
#[derive(Debug)]
struct Location {
    folder: File,
    name: Component,
}
impl Root {
    fn location(&self, path: &CheckpointPath, policy: ParentPolicy) -> Result<Location, Error> {
        self.same_root()?;
        let mut folder = self.file.try_clone()?;
        let mut parts = path.relative().parts().peekable();
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                return Ok(Location {
                    folder,
                    name: Component(part.to_owned()),
                });
            }
            let child = match rustix::fs::openat(&folder, part, DIRECTORY, Mode::empty()) {
                Ok(fd) => File::from(fd),
                Err(rustix::io::Errno::NOENT) if policy == ParentPolicy::CreateMissing => {
                    rustix::fs::mkdirat(&folder, part, Mode::RWXU).map_err(io::Error::from)?;
                    let child = File::from(
                        rustix::fs::openat(&folder, part, DIRECTORY, Mode::empty())
                            .map_err(io::Error::from)?,
                    );
                    rustix::fs::fchmod(&child, Mode::RWXU).map_err(io::Error::from)?;
                    folder.sync_all()?;
                    child
                }
                Err(rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) => {
                    return Err(CheckpointError::UnsafeEntry.into());
                }
                Err(error) => return Err(io::Error::from(error).into()),
            };
            let found = child.metadata()?;
            use std::os::unix::fs::MetadataExt as _;
            if found.dev() != self.identity.device || found.uid() != self.identity.owner {
                return Err(CheckpointError::UnsupportedMetadata.into());
            }
            refuse_attributes(&child)?;
            folder = child;
        }
        Err(CheckpointError::InvalidPath.into())
    }
    fn same_root(&self) -> Result<(), Error> {
        let current = Self::open(&self.path)?;
        if current.identity.device != self.identity.device
            || current.identity.inode != self.identity.inode
            || current.identity.owner != self.identity.owner
            || current.identity.group != self.identity.group
            || current.identity.mode != self.identity.mode
        {
            return Err(CheckpointError::CaptureUnstable.into());
        }
        Ok(())
    }
    pub(in crate::checkpoint) fn install_create(
        &self,
        path: &CheckpointPath,
        stage: &Stage,
        slot: Slot,
        policy: ParentPolicy,
    ) -> Result<(), Error> {
        let target = self.location(path, policy)?;
        let source_metadata = stage.folder.metadata()?;
        let destination_metadata = target.folder.metadata()?;
        use std::os::unix::fs::MetadataExt as _;
        if source_metadata.dev() != destination_metadata.dev() {
            return Err(CheckpointError::UnsupportedScope.into());
        }
        rustix::fs::renameat_with(
            &stage.folder,
            slot.text(),
            &target.folder,
            target.name.0,
            RenameFlags::NOREPLACE,
        )
        .map_err(io::Error::from)?;
        target.folder.sync_all()?;
        stage.folder.sync_all()?;
        Ok(())
    }
    pub(in crate::checkpoint) fn exchange(
        &self,
        path: &CheckpointPath,
        stage: &Stage,
        slot: Slot,
    ) -> Result<(), Error> {
        let target = self.location(path, ParentPolicy::Existing)?;
        rustix::fs::renameat_with(
            &stage.folder,
            slot.text(),
            &target.folder,
            target.name.0,
            RenameFlags::EXCHANGE,
        )
        .map_err(io::Error::from)?;
        target.folder.sync_all()?;
        stage.folder.sync_all()?;
        Ok(())
    }
    pub(in crate::checkpoint) fn take(
        &self,
        path: &CheckpointPath,
        stage: &Stage,
        slot: Slot,
    ) -> Result<(), Error> {
        let source = self.location(path, ParentPolicy::Existing)?;
        rustix::fs::renameat_with(
            &source.folder,
            source.name.0,
            &stage.folder,
            slot.text(),
            RenameFlags::NOREPLACE,
        )
        .map_err(io::Error::from)?;
        source.folder.sync_all()?;
        stage.folder.sync_all()?;
        Ok(())
    }
}
