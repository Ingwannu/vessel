//! One bounded hashing/copy loop for capture, restore staging and verification.
use super::Error;
use sha2::{Digest as _, Sha256};
use std::{
    fs::File,
    io::{Read as _, Write},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    BlobHash,
    checkpoint::{CHECKPOINT_IO_CHUNK, CheckpointError, CheckpointFileBytes},
};

#[derive(Debug, Clone, Copy)]
pub(super) enum Budget {
    ImageRemaining(CheckpointFileBytes),
    Exact(CheckpointFileBytes),
}
impl Budget {
    fn check(self, total: CheckpointFileBytes) -> Result<(), CheckpointError> {
        match self {
            Self::ImageRemaining(maximum) if total > maximum => Err(CheckpointError::ImageTooLarge),
            Self::Exact(expected) if total > expected => Err(CheckpointError::CaptureUnstable),
            Self::ImageRemaining(_) | Self::Exact(_) => Ok(()),
        }
    }
    fn finish(self, total: CheckpointFileBytes) -> Result<(), CheckpointError> {
        match self {
            Self::Exact(expected) if total != expected => Err(CheckpointError::CaptureUnstable),
            Self::ImageRemaining(_) | Self::Exact(_) => Ok(()),
        }
    }
}
pub(super) fn transfer(
    source: &mut File,
    destination: &mut impl Write,
    budget: Budget,
    cancellation: &CancellationToken,
) -> Result<(BlobHash, CheckpointFileBytes), Error> {
    let mut buffer = vec![0_u8; CHECKPOINT_IO_CHUNK].into_boxed_slice();
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
        budget.check(CheckpointFileBytes::new(total)?)?;
        let bytes = buffer.get(..count).ok_or(CheckpointError::FileTooLarge)?;
        hash.update(bytes);
        destination.write_all(bytes)?;
    }
    let total = CheckpointFileBytes::new(total)?;
    budget.finish(total)?;
    Ok((BlobHash::from_bytes(hash.finalize().into()), total))
}
