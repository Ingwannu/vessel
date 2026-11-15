//! Binary file pages share the existing checked root and regular-file open.
//! Hashing the bounded source for every page costs I/O, but makes a
//! transfer fail rather than silently concatenate different revisions.

use std::io::{Read, Seek};
mod pass;
#[cfg(test)]
mod race_tests;

use vessel_proto::workspace::RelPath;
use vessel_proto::workspace::panel_files::{
    FileBytePage, FileBytes, FileOffset, FileRevision, MAX_FILE_BYTES, ObservedFile,
};

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
        if file.metadata().map_err(FileError::Io)?.len() > MAX_FILE_BYTES {
            return Err(FileError::TransferTooLarge);
        }
        read_page(&mut file, &path, offset, expected.as_ref())
    })
    .await
}

fn read_page<R: Read + Seek>(
    file: &mut R,
    path: &RelPath,
    offset: FileOffset,
    expected: Option<&FileRevision>,
) -> Result<FileBytePage, FileError> {
    // Capture each page byte from the very buffer fed to the digest. Separate
    // before/after hashes admit A→B→A substitution of the intervening page.
    let snapshot = pass::read(file, Some(offset))?;
    let revision = snapshot.revision;
    if expected.is_some_and(|at| at != &revision) || (offset.get() != 0 && expected.is_none()) {
        return Err(FileError::Changed(path.clone()));
    }
    if offset.get() > revision.size.0 {
        return Err(FileError::Offset);
    }
    let end = offset
        .get()
        .checked_add(u64::try_from(snapshot.page.len()).map_err(|_invalid| FileError::Task)?)
        .ok_or(FileError::Offset)?;
    Ok(FileBytePage {
        next: if end < revision.size.0 {
            Some(FileOffset::try_from(end).map_err(|_invalid| FileError::Offset)?)
        } else {
            None
        },
        revision,
        offset,
        bytes: FileBytes::new(snapshot.page.into()).map_err(|_invalid| FileError::Task)?,
    })
}

/// Saved-state hashing shares the same bounded read pass, without retaining a page.
pub(super) fn revision(file: &mut std::fs::File) -> Result<FileRevision, FileError> {
    if file.metadata().map_err(FileError::Io)?.len() > MAX_FILE_BYTES {
        return Err(FileError::TransferTooLarge);
    }
    Ok(pass::read(file, None)?.revision)
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
    use vessel_proto::workspace::panel_files::MAX_PAGE_BYTES;

    #[tokio::test]
    async fn binary_pages_are_exact_and_reject_changed_or_unguarded_continuations() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::open(temp.path()).unwrap();
        let path: RelPath = "data.pdf".parse().unwrap();
        let mut source = vec![0xff; MAX_PAGE_BYTES + 5];
        source[0] = 0;
        std::fs::write(temp.path().join("data.pdf"), &source).unwrap();
        let first = read_bytes(&root, &path, FileOffset::ZERO, None)
            .await
            .unwrap();
        assert_eq!(first.bytes.bytes().len(), MAX_PAGE_BYTES);
        let unaligned = read_bytes(
            &root,
            &path,
            FileOffset::try_from(3).unwrap(),
            Some(&first.revision),
        )
        .await
        .unwrap();
        assert_eq!(
            unaligned.bytes.bytes().as_ref(),
            &source[3..MAX_PAGE_BYTES + 3]
        );
        assert_eq!(
            unaligned.next.unwrap().get(),
            u64::try_from(MAX_PAGE_BYTES + 3).unwrap()
        );

        let second = read_bytes(&root, &path, first.next.unwrap(), Some(&first.revision))
            .await
            .unwrap();
        assert_eq!(second.bytes.bytes().as_ref(), &source[MAX_PAGE_BYTES..]);
        assert!(second.next.is_none());
        assert!(
            read_bytes(&root, &path, FileOffset::try_from(1).unwrap(), None)
                .await
                .is_err()
        );
        std::fs::write(temp.path().join("data.pdf"), b"changed").unwrap();
        assert!(matches!(
            read_bytes(&root, &path, FileOffset::ZERO, Some(&first.revision)).await,
            Err(FileError::Changed(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn external_and_git_links_are_refusals_not_missing_snapshots() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("private"), "private").unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        std::fs::write(temp.path().join(".git/config"), "git").unwrap();
        std::os::unix::fs::symlink(outside.path().join("private"), temp.path().join("external"))
            .unwrap();
        std::os::unix::fs::symlink(
            temp.path().join(".git/config"),
            temp.path().join("git-link"),
        )
        .unwrap();
        let root = Root::open(temp.path()).unwrap();
        for name in ["external", "git-link"] {
            let path = name.parse().unwrap();
            assert!(
                read_bytes(&root, &path, FileOffset::ZERO, None)
                    .await
                    .is_err()
            );
            assert!(observed(&root, &path).await.is_err());
        }
        assert_eq!(
            observed(&root, &"missing".parse().unwrap()).await.unwrap(),
            ObservedFile::Missing
        );
    }

    #[tokio::test]
    async fn oversized_source_and_past_eof_offsets_refuse_before_a_page() {
        let temp = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(temp.path().join("large")).unwrap();
        file.set_len(MAX_FILE_BYTES + 1).unwrap();
        let root = Root::open(temp.path()).unwrap();
        assert!(matches!(
            read_bytes(&root, &"large".parse().unwrap(), FileOffset::ZERO, None).await,
            Err(FileError::TransferTooLarge)
        ));
        std::fs::write(temp.path().join("small"), "a").unwrap();
        let path = "small".parse().unwrap();
        let first = read_bytes(&root, &path, FileOffset::ZERO, None)
            .await
            .unwrap();
        assert!(matches!(
            read_bytes(
                &root,
                &path,
                FileOffset::try_from(2).unwrap(),
                Some(&first.revision)
            )
            .await,
            Err(FileError::Offset)
        ));
    }
}
