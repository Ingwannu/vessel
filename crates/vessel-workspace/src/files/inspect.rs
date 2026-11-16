//! Modification metadata is read without following leaf links. Missing
//! entries are omitted; unrelated disk failures stay visible.

use std::fs;
use std::time::UNIX_EPOCH;

use vessel_proto::workspace::EntryKind;
use vessel_proto::workspace::directory_metadata::{
    FileModifiedMillis, IgnoreState, IgnoredEntries, InspectDirectory, InspectedDirectory,
    InspectedEntry,
};
use vessel_proto::workspace::panel_files::FileSize;

use super::{FileError, blocking, list, list_marked};
use crate::{Git, paths::Root};

/// Inspect the bounded ordinary listing inside the same checked root.
///
/// # Errors
/// Lists, path checks and metadata can fail; no timestamp is fabricated.
pub async fn inspect(
    git: Option<&Git>,
    root: &Root,
    request: &InspectDirectory,
) -> Result<InspectedDirectory, FileError> {
    let listing = match git {
        Some(git) => {
            list_marked(
                git,
                root,
                &request.directory,
                request.ignored == IgnoredEntries::Include,
            )
            .await?
        }
        None => list(root, &request.directory).await?,
    };
    let (root, directory) = (root.clone(), request.directory.clone());
    blocking(move || {
        let mut entries = Vec::with_capacity(listing.entries.len());
        for entry in listing.entries {
            let path = directory
                .child(&entry.name)
                .map_err(FileError::InvalidEntry)?;
            let target = root.entry(&path)?;
            let metadata = match fs::symlink_metadata(&target.path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(FileError::Io(error)),
            };
            let modified = metadata
                .modified()
                .ok()
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
                .and_then(|duration| u64::try_from(duration.as_millis()).ok())
                .map(FileModifiedMillis);
            let kind = if metadata.file_type().is_symlink() {
                EntryKind::Symlink
            } else if metadata.is_dir() {
                EntryKind::Dir
            } else if metadata.is_file() {
                EntryKind::File
            } else {
                EntryKind::Other
            };
            entries.push(InspectedEntry {
                path,
                kind,
                size: (kind == EntryKind::File).then_some(FileSize(metadata.len())),
                modified,
                ignored: if entry.ignored {
                    IgnoreState::Ignored
                } else {
                    IgnoreState::Visible
                },
            });
        }
        Ok(InspectedDirectory {
            entries,
            more: listing.more,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{
        ProjectId,
        workspace::{RelPath, WorkspaceRoot},
    };

    #[tokio::test]
    async fn metadata_contains_real_size_and_modification_time() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("file"), b"bytes").unwrap();
        let root = Root::open(temp.path()).unwrap();
        let request = InspectDirectory {
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
            directory: RelPath::root(),
            ignored: IgnoredEntries::Include,
        };
        let found = inspect(None, &root, &request).await.unwrap();
        assert_eq!(found.entries.len(), 1);
        let file = found.entries.first().unwrap();
        assert_eq!(file.size, Some(FileSize(5)));
        assert!(file.modified.is_some());
    }
}
