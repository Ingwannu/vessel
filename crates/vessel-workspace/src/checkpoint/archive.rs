//! Handle-relative private immutable blobs with streaming allocation and byte bounds.

use super::{
    Error,
    rooted::{Identity, refuse_attributes},
};
use rustix::fs::{AtFlags, CWD, Mode, OFlags};
use sha2::{Digest as _, Sha256};
use std::{
    fs::File,
    io::{Read as _, Write as _},
    os::unix::fs::MetadataExt as _,
    path::PathBuf,
};
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    BlobHash,
    checkpoint::{CHECKPOINT_IO_CHUNK, CheckpointError, FileBytes},
};

const PRIVATE_MODE: u32 = 0o600;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const ORDINARY_BITS: u32 = 0o777;
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
    path: PathBuf,
}
impl Archive {
    pub(super) fn open(path: PathBuf) -> Result<Self, Error> {
        let folder = File::from(
            rustix::fs::openat(CWD, &path, DIRECTORY, Mode::empty())
                .map_err(std::io::Error::from)?,
        );
        let metadata = folder.metadata()?;
        if metadata.mode() & ORDINARY_BITS != PRIVATE_DIRECTORY_MODE {
            return Err(CheckpointError::UnsupportedMetadata.into());
        }
        refuse_attributes(&folder)?;
        Ok(Self { folder, path })
    }
    /// Private per-capture folder, never a shared temporary filename namespace.
    pub(super) fn capture(
        &self,
        source: &mut File,
        budget: FileBytes,
        cancellation: &CancellationToken,
    ) -> Result<(BlobHash, FileBytes), Error> {
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
        budget: FileBytes,
        cancellation: &CancellationToken,
    ) -> Result<(BlobHash, FileBytes), Error> {
        let (digest, bytes) = stream(source, output, budget, cancellation)?;
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
            Err(rustix::io::Errno::EXIST) => self.verify(digest, bytes)?,
            Err(error) => return Err(std::io::Error::from(error).into()),
        }
        self.folder.sync_all()?;
        Ok((digest, bytes))
    }
    pub(super) fn path(&self, digest: BlobHash) -> PathBuf {
        self.path.join(digest.to_string())
    }
    fn verify(&self, digest: BlobHash, bytes: FileBytes) -> Result<(), Error> {
        let mut source = File::from(
            rustix::fs::openat(&self.folder, digest.to_string(), READ_FILE, Mode::empty())
                .map_err(std::io::Error::from)?,
        );
        let before = source.metadata()?;
        if !before.is_file()
            || before.nlink() != 1
            || before.len() != bytes.get()
            || before.uid() != self.folder.metadata()?.uid()
            || before.mode() & ORDINARY_BITS != PRIVATE_MODE
        {
            return Err(CheckpointError::UnsafeEntry.into());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0_u8; CHECKPOINT_IO_CHUNK];
        let mut total = 0_u64;
        loop {
            let count = source.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            total = total
                .checked_add(u64::try_from(count).map_err(|_large| CheckpointError::FileTooLarge)?)
                .ok_or(CheckpointError::FileTooLarge)?;
            if total > bytes.get() {
                return Err(CheckpointError::CaptureUnstable.into());
            }
            hash.update(buffer.get(..count).ok_or(CheckpointError::FileTooLarge)?);
        }
        if total != bytes.get()
            || BlobHash::from_bytes(hash.finalize().into()) != digest
            || Identity::of(&source.metadata()?) != Identity::of(&before)
        {
            return Err(CheckpointError::CaptureUnstable.into());
        }
        Ok(())
    }
}
fn stream(
    source: &mut File,
    destination: &mut File,
    budget: FileBytes,
    cancellation: &CancellationToken,
) -> Result<(BlobHash, FileBytes), Error> {
    let mut buffer = [0_u8; CHECKPOINT_IO_CHUNK];
    let mut total = 0_u64;
    let mut hash = Sha256::new();
    loop {
        if cancellation.is_cancelled() {
            return Err(CheckpointError::CaptureTimedOut.into());
        }
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(count).map_err(|_large| CheckpointError::FileTooLarge)?)
            .ok_or(CheckpointError::FileTooLarge)?;
        let _bounded = FileBytes::new(total)?;
        if total > budget.get() {
            return Err(CheckpointError::ImageTooLarge.into());
        }
        let chunk = buffer.get(..count).ok_or(CheckpointError::FileTooLarge)?;
        hash.update(chunk);
        destination.write_all(chunk)?;
    }
    Ok((
        BlobHash::from_bytes(hash.finalize().into()),
        FileBytes::new(total)?,
    ))
}
