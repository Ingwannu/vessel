//! Raw immutable blobs are private, streamed and never addressed by untrusted paths.

use super::{Error, rooted::Identity};
use sha2::{Digest as _, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read as _, Write as _},
    os::unix::fs::{MetadataExt as _, OpenOptionsExt as _},
    path::{Path, PathBuf},
};
use vessel_proto::{
    BlobHash,
    checkpoint::{CHECKPOINT_IO_CHUNK, CheckpointError, FileBytes},
};

const PRIVATE_MODE: u32 = 0o600;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const ORDINARY_BITS: u32 = 0o777;

#[derive(Debug)]
pub(super) struct Archive {
    root: PathBuf,
}
impl Archive {
    pub(super) fn open(root: PathBuf) -> Result<Self, Error> {
        let metadata = std::fs::symlink_metadata(&root)?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.mode() & ORDINARY_BITS != PRIVATE_DIRECTORY_MODE
        {
            return Err(CheckpointError::UnsupportedMetadata.into());
        }
        Ok(Self { root })
    }
    /// The caller provides a private per-capture directory. No shared temp name.
    pub(super) fn capture(&self, source: &mut File) -> Result<(BlobHash, FileBytes), Error> {
        let before = Identity::of(&source.metadata()?);
        let temp = self.root.join("candidate");
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(PRIVATE_MODE)
            .open(&temp)?;
        let result = stream(source, &mut output);
        let result = result.and_then(|(digest, bytes)| {
            if Identity::of(&source.metadata()?) != before {
                return Err(CheckpointError::CaptureUnstable.into());
            }
            output.sync_all()?;
            let destination = self.root.join(digest.to_string());
            match std::fs::hard_link(&temp, &destination) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    verify(&destination, digest, bytes)?
                }
                Err(error) => return Err(error.into()),
            }
            File::open(&self.root)?.sync_all()?;
            Ok((digest, bytes))
        });
        drop(output);
        let cleaned = std::fs::remove_file(temp);
        match (result, cleaned) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error.into()),
        }
    }
    pub(super) fn path(&self, digest: BlobHash) -> PathBuf {
        self.root.join(digest.to_string())
    }
}
fn stream(source: &mut File, destination: &mut File) -> Result<(BlobHash, FileBytes), Error> {
    let mut buffer = [0_u8; CHECKPOINT_IO_CHUNK];
    let mut total = 0_u64;
    let mut hash = Sha256::new();
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(count).map_err(|_large| CheckpointError::FileTooLarge)?)
            .ok_or(CheckpointError::FileTooLarge)?;
        let _bounded = FileBytes::new(total)?;
        let chunk = buffer.get(..count).ok_or(CheckpointError::FileTooLarge)?;
        hash.update(chunk);
        destination.write_all(chunk)?;
    }
    Ok((
        BlobHash::from_bytes(hash.finalize().into()),
        FileBytes::new(total)?,
    ))
}
fn verify(path: &Path, digest: BlobHash, bytes: FileBytes) -> Result<(), Error> {
    let mut source = crate::regular::open(path)?.ok_or(CheckpointError::UnsafeEntry)?;
    if source.metadata()?.len() != bytes.get() {
        return Err(CheckpointError::CaptureUnstable.into());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; CHECKPOINT_IO_CHUNK];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(buffer.get(..count).ok_or(CheckpointError::FileTooLarge)?);
    }
    if BlobHash::from_bytes(hash.finalize().into()) != digest {
        return Err(CheckpointError::CaptureUnstable.into());
    }
    Ok(())
}
