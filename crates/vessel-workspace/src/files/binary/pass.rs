//! One bounded byte stream supplies both digest and captured page. This is a
//! read-pass identity, not a claim that a mutable file is an atomic snapshot.

use super::FileError;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom};
use vessel_proto::workspace::ContentHash;
use vessel_proto::workspace::panel_files::{
    FileOffset, FileRevision, FileSize, MAX_FILE_BYTES, MAX_PAGE_BYTES,
};

#[derive(Debug)]
pub(super) struct ReadPass {
    pub(super) revision: FileRevision,
    pub(super) page: Vec<u8>,
}

pub(super) fn read<R: Read + Seek>(
    file: &mut R,
    offset: Option<FileOffset>,
) -> Result<ReadPass, FileError> {
    let _position = file.seek(SeekFrom::Start(0)).map_err(FileError::Io)?;
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    let mut page = Vec::new();
    let mut buffer = vec![0; MAX_PAGE_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(FileError::Io)?;
        if read == 0 {
            break;
        }
        let start = size;
        size = size
            .checked_add(u64::try_from(read).map_err(|_invalid| FileError::Task)?)
            .ok_or(FileError::TransferTooLarge)?;
        if size > MAX_FILE_BYTES {
            return Err(FileError::TransferTooLarge);
        }
        let chunk = buffer.get(..read).ok_or(FileError::Task)?;
        hash.update(chunk);
        if let Some(offset) = offset {
            capture(chunk, start, offset, &mut page)?;
        }
    }
    Ok(ReadPass {
        revision: FileRevision {
            size: FileSize(size),
            hash: ContentHash::from_sha256(hash.finalize().into()),
        },
        page,
    })
}

fn capture(
    chunk: &[u8],
    start: u64,
    offset: FileOffset,
    page: &mut Vec<u8>,
) -> Result<(), FileError> {
    let width = u64::try_from(MAX_PAGE_BYTES).map_err(|_invalid| FileError::Task)?;
    let page_end = offset.get().checked_add(width).ok_or(FileError::Offset)?;
    let chunk_end = start
        .checked_add(u64::try_from(chunk.len()).map_err(|_invalid| FileError::Task)?)
        .ok_or(FileError::Offset)?;
    let first = start.max(offset.get());
    let last = chunk_end.min(page_end);
    if first < last {
        let first = usize::try_from(first.checked_sub(start).ok_or(FileError::Offset)?)
            .map_err(|_invalid| FileError::Offset)?;
        let last = usize::try_from(last.checked_sub(start).ok_or(FileError::Offset)?)
            .map_err(|_invalid| FileError::Offset)?;
        page.extend_from_slice(chunk.get(first..last).ok_or(FileError::Offset)?);
    }
    Ok(())
}
