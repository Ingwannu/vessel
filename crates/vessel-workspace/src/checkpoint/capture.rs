//! Per-file images use opened identity, content, and effective metadata evidence.
use super::{
    Error,
    archive::Archive,
    rooted::{Root, metadata},
};
use vessel_proto::checkpoint::{CheckpointPath, Entry, FileImage, FileState};

pub(super) fn image(
    root: &Root,
    archive: &Archive,
    paths: &[CheckpointPath],
) -> Result<FileImage, Error> {
    let mut entries = Vec::new();
    for path in paths {
        let state = match root.file(path)? {
            None => FileState::Absent,
            Some(mut file) => {
                let metadata = metadata(&file)?;
                let (digest, bytes) = archive.capture(&mut file)?;
                FileState::Present {
                    digest,
                    bytes,
                    metadata,
                }
            }
        };
        entries.push(Entry {
            path: path.clone(),
            state,
        });
    }
    Ok(FileImage::new(entries)?)
}
