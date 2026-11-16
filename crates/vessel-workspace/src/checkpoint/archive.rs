//! Handle-relative private immutable blobs with streaming allocation and byte bounds.

use super::{
    Error,
    content::{self, Budget},
    rooted::{Identity, refuse_attributes},
};
use rustix::fs::{AtFlags, CWD, Mode, OFlags};
use std::{fs::File, os::unix::fs::MetadataExt as _, path::PathBuf};
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    BlobHash,
    checkpoint::{CheckpointError, CheckpointFileBytes},
};

const PRIVATE_MODE: u32 = 0o600;
const PERMISSION_BITS: u32 = 0o7777;
const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const NEW_FILE: OFlags = OFlags::WRONLY
    .union(OFlags::CREATE)
    .union(OFlags::EXCL)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const READ_FILE: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::NONBLOCK)
    .union(OFlags::CLOEXEC);

#[derive(Debug)]
pub(super) struct Archive {
    folder: File,
    #[cfg(test)]
    path: PathBuf,
}
impl Archive {
    pub(super) fn open(path: PathBuf) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(CheckpointError::InvalidPath.into());
        }
        let folder = File::from(
            rustix::fs::openat(CWD, &path, DIRECTORY, Mode::empty())
                .map_err(std::io::Error::from)?,
        );
        Self::from_folder(folder, path)
    }
    pub(in crate::checkpoint) fn from_folder(folder: File, path: PathBuf) -> Result<Self, Error> {
        super::account::private_directory(&folder)?;
        #[cfg(not(test))]
        drop(path);
        Ok(Self {
            folder,
            #[cfg(test)]
            path,
        })
    }
    /// Private per-capture folder, never a shared temporary filename namespace.
    pub(super) fn capture(
        &self,
        source: &mut File,
        budget: CheckpointFileBytes,
        cancellation: &CancellationToken,
    ) -> Result<(BlobHash, CheckpointFileBytes), Error> {
        let before = Identity::of(&source.metadata()?);
        let mut output = File::from(
            rustix::fs::openat(
                &self.folder,
                "candidate",
                NEW_FILE,
                Mode::from_raw_mode(PRIVATE_MODE),
            )
            .map_err(std::io::Error::from)?,
        );
        rustix::fs::fchmod(&output, Mode::from_raw_mode(PRIVATE_MODE))
            .map_err(std::io::Error::from)?;
        let result = self.capture_to(source, &mut output, before, budget, cancellation);
        drop(output);
        let cleaned = rustix::fs::unlinkat(&self.folder, "candidate", AtFlags::empty())
            .map_err(std::io::Error::from);
        match (result, cleaned) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error.into()),
        }
    }
    fn capture_to(
        &self,
        source: &mut File,
        output: &mut File,
        before: Identity,
        budget: CheckpointFileBytes,
        cancellation: &CancellationToken,
    ) -> Result<(BlobHash, CheckpointFileBytes), Error> {
        let (digest, bytes) =
            content::transfer(source, output, Budget::ImageRemaining(budget), cancellation)?;
        if Identity::of(&source.metadata()?) != before {
            return Err(CheckpointError::CaptureUnstable.into());
        }
        output.sync_all()?;
        match rustix::fs::linkat(
            &self.folder,
            "candidate",
            &self.folder,
            digest.to_string(),
            AtFlags::empty(),
        ) {
            Ok(()) => {}
            Err(rustix::io::Errno::EXIST) => self.verify(digest, bytes, cancellation)?,
            Err(error) => return Err(std::io::Error::from(error).into()),
        }
        self.folder.sync_all()?;
        Ok((digest, bytes))
    }
    #[cfg(test)]
    pub(super) fn path(&self, digest: BlobHash) -> PathBuf {
        self.path.join(digest.to_string())
    }
    pub(super) fn open_blob(&self, digest: BlobHash) -> Result<File, Error> {
        let source = File::from(
            rustix::fs::openat(&self.folder, digest.to_string(), READ_FILE, Mode::empty())
                .map_err(std::io::Error::from)?,
        );
        let found = source.metadata()?;
        if !found.is_file()
            || found.nlink() != 1
            || found.uid() != self.folder.metadata()?.uid()
            || found.mode() & PERMISSION_BITS != PRIVATE_MODE
        {
            return Err(CheckpointError::UnsafeEntry.into());
        }
        refuse_attributes(&source)?;
        if super::rooted::MountId::of(&source)? != super::rooted::MountId::of(&self.folder)? {
            return Err(CheckpointError::UnsafeEntry.into());
        }
        Ok(source)
    }
    pub(super) fn verify(
        &self,
        digest: BlobHash,
        bytes: CheckpointFileBytes,
        cancellation: &CancellationToken,
    ) -> Result<(), Error> {
        let mut source = self.open_blob(digest)?;
        let before = source.metadata()?;
        if before.len() != bytes.get() {
            return Err(CheckpointError::CaptureUnstable.into());
        }
        let (found, _bytes) = content::transfer(
            &mut source,
            &mut std::io::sink(),
            Budget::Exact(bytes),
            cancellation,
        )?;
        if found != digest || Identity::of(&source.metadata()?) != Identity::of(&before) {
            return Err(CheckpointError::CaptureUnstable.into());
        }
        Ok(())
    }
}
