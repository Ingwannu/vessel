//! Verification reads the exact regular file through its already-open root.
use super::{
    Error,
    content::{self, Budget},
    rooted::{Identity, Root, metadata},
};
use std::fs::File;
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{CheckpointError, CheckpointFileBytes, CheckpointPath, FileState};

pub(super) fn file(
    source: &mut File,
    cancellation: &CancellationToken,
) -> Result<FileState, Error> {
    let before = source.metadata()?;
    let identity = Identity::of(&before);
    let metadata = metadata(source)?;
    let (digest, bytes) = content::transfer(
        source,
        &mut std::io::sink(),
        Budget::Exact(CheckpointFileBytes::new(before.len())?),
        cancellation,
    )?;
    if Identity::of(&source.metadata()?) != identity {
        return Err(CheckpointError::CaptureUnstable.into());
    }
    Ok(FileState::Present {
        digest,
        bytes,
        metadata,
    })
}
pub(super) fn path(
    root: &Root,
    path: &CheckpointPath,
    cancellation: &CancellationToken,
) -> Result<FileState, Error> {
    match root.file(path)? {
        None => Ok(FileState::Absent),
        Some(mut source) => file(&mut source, cancellation),
    }
}
