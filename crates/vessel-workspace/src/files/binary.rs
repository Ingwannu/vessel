//! Binary file pages share the existing checked root and regular-file open.
//! Hashing the bounded source for every page costs I/O, but makes a
//! transfer fail rather than silently concatenate different revisions.

use std::io::{Read as _, Seek as _, SeekFrom};

use sha2::{Digest as _, Sha256};
use vessel_proto::workspace::panel_files::{
    FileBytePage, FileBytes, FileOffset, FileRevision, FileSize, MAX_FILE_BYTES, MAX_PAGE_BYTES,
    ObservedFile,
};
use vessel_proto::workspace::{ContentHash, RelPath};

use super::{FileError, blocking, open_file};
use crate::paths::{PathError, Root};

/// Read exact bytes, requiring the initial revision on every continuation.
///
/// # Errors
/// Refuses mixed transfers, special files, oversized sources and bad offsets.
pub async fn read_bytes(
    root: &Root,
    path: &RelPath,
    offset: FileOffset,
    expected: Option<&FileRevision>,
) -> Result<FileBytePage, FileError> {
    let (root, path, expected) = (root.clone(), path.clone(), expected.cloned());
    blocking(move || {
        let full = root.existing(&path)?;
        let mut file = open_file(&full, &path)?;
        let revision = revision(&mut file)?;
        if expected.as_ref().is_some_and(|at| at != &revision)
            || (offset.0 != 0 && expected.is_none())
        {
            return Err(FileError::Changed(path));
        }
        if offset.0 > revision.size.0 {
            return Err(FileError::Offset);
        }
        let _position = file
            .seek(SeekFrom::Start(offset.0))
            .map_err(FileError::Io)?;
        let mut bytes = Vec::new();
        let limit = u64::try_from(MAX_PAGE_BYTES).map_err(|_invalid| FileError::Task)?;
        let _read = file
            .by_ref()
            .take(limit)
            .read_to_end(&mut bytes)
            .map_err(FileError::Io)?;
        // Verify after reading too: the page's bytes must belong to the
        // content that was hashed before it was read.
        if self::revision(&mut file)? != revision {
            return Err(FileError::Changed(path));
        }
        let end = offset
            .0
            .saturating_add(u64::try_from(bytes.len()).map_err(|_invalid| FileError::Task)?);
        if end > revision.size.0 || (end < revision.size.0 && bytes.is_empty()) {
            return Err(FileError::Changed(path));
        }
        Ok(FileBytePage {
            next: (end < revision.size.0).then_some(FileOffset(end)),
            revision,
            offset,
            bytes: FileBytes::new(bytes.into()).map_err(|_invalid| FileError::Task)?,
        })
    })
    .await
}

/// A complete hash, bounded even when a file grows during hashing.
pub(super) fn revision(file: &mut std::fs::File) -> Result<FileRevision, FileError> {
    if file.metadata().map_err(FileError::Io)?.len() > MAX_FILE_BYTES {
        return Err(FileError::TransferTooLarge);
    }
    let _position = file.seek(SeekFrom::Start(0)).map_err(FileError::Io)?;
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = vec![0; MAX_PAGE_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(FileError::Io)?;
        if read == 0 {
            break;
        }
        size = size.saturating_add(u64::try_from(read).map_err(|_invalid| FileError::Task)?);
        if size > MAX_FILE_BYTES {
            return Err(FileError::TransferTooLarge);
        }
        hash.update(buffer.get(..read).ok_or(FileError::Task)?);
    }
    Ok(FileRevision {
        size: FileSize(size),
        hash: ContentHash::from_sha256(hash.finalize().into()),
    })
}

/// Read saved state without turning an outside/symlink refusal into absence.
///
/// # Errors
/// As [`read_bytes`]; a missing regular file is an explicit state.
pub async fn observed(root: &Root, path: &RelPath) -> Result<ObservedFile, FileError> {
    let (root, path) = (root.clone(), path.clone());
    blocking(move || {
        let full = match root.existing(&path) {
            Ok(full) => full,
            Err(PathError::NotFound(_)) => return Ok(ObservedFile::Missing),
            Err(error) => return Err(error.into()),
        };
        let mut file = open_file(&full, &path)?;
        Ok(ObservedFile::Ready {
            revision: revision(&mut file)?,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn binary_pages_are_exact_and_reject_changed_or_unguarded_continuations() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::open(temp.path()).unwrap();
        let path: RelPath = "data.pdf".parse().unwrap();
        let mut source = vec![0xff; MAX_PAGE_BYTES + 5];
        source[0] = 0;
        std::fs::write(temp.path().join("data.pdf"), &source).unwrap();
        let first = read_bytes(&root, &path, FileOffset(0), None).await.unwrap();
        assert_eq!(first.bytes.bytes().len(), MAX_PAGE_BYTES);
        let second = read_bytes(&root, &path, first.next.unwrap(), Some(&first.revision))
            .await
            .unwrap();
        assert_eq!(second.bytes.bytes().as_ref(), &source[MAX_PAGE_BYTES..]);
        assert!(second.next.is_none());
        assert!(read_bytes(&root, &path, FileOffset(1), None).await.is_err());
        std::fs::write(temp.path().join("data.pdf"), b"changed").unwrap();
        assert!(matches!(
            read_bytes(&root, &path, FileOffset(0), Some(&first.revision)).await,
            Err(FileError::Changed(_))
        ));
    }
}
