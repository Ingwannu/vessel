//! Per-file images bound aggregate bytes while streaming and recheck file identities.
use super::{
    Error,
    archive::Archive,
    rooted::{Identity, Root, metadata},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CHECKPOINT_FILE_BYTES, CHECKPOINT_IMAGE_BYTES, CheckpointError, CheckpointPath, Entry,
    FileBytes, FileImage, FileState, IgnoreFingerprint, ObservedEntry, ObservedSnapshot,
    SnapshotEvidence,
};

#[derive(Debug, Clone, Copy)]
pub(super) enum Destination<'a> {
    Archive(&'a Archive),
    Observe,
}
#[cfg(test)]
pub(super) fn image(
    root: &Root,
    archive: &Archive,
    paths: &[CheckpointPath],
    cancellation: &CancellationToken,
) -> Result<FileImage, Error> {
    image_to(root, Destination::Archive(archive), paths, cancellation)
}
fn image_to(
    root: &Root,
    destination: Destination<'_>,
    paths: &[CheckpointPath],
    cancellation: &CancellationToken,
) -> Result<FileImage, Error> {
    let mut entries = Vec::new();
    let mut identities = Vec::new();
    let mut total = 0_u64;
    for path in paths {
        if cancellation.is_cancelled() {
            return Err(CheckpointError::CaptureTimedOut.into());
        }
        let (state, identity) = match root.file(path)? {
            None => (FileState::Absent, None),
            Some(mut file) => {
                let identity = Identity::of(&file.metadata()?);
                let metadata = metadata(&file)?;
                let budget = FileBytes::new(
                    CHECKPOINT_IMAGE_BYTES
                        .saturating_sub(total)
                        .min(CHECKPOINT_FILE_BYTES),
                )?;
                let (digest, bytes) = match destination {
                    Destination::Archive(archive) => {
                        archive.capture(&mut file, budget, cancellation)?
                    }
                    Destination::Observe => super::content::transfer(
                        &mut file,
                        &mut std::io::sink(),
                        super::content::Budget::ImageRemaining(budget),
                        cancellation,
                    )?,
                };
                if Identity::of(&file.metadata()?) != identity {
                    return Err(CheckpointError::CaptureUnstable.into());
                }
                total = total
                    .checked_add(bytes.get())
                    .ok_or(CheckpointError::ImageTooLarge)?;
                (
                    FileState::Present {
                        digest,
                        bytes,
                        metadata,
                    },
                    Some(identity),
                )
            }
        };
        identities.push((path, identity));
        entries.push(Entry {
            path: path.clone(),
            state,
        });
    }
    // Earlier reads must still refer to the same files before accepting the image.
    for (path, before) in identities {
        let after = root
            .file(path)?
            .map(|file| file.metadata())
            .transpose()?
            .as_ref()
            .map(Identity::of);
        if after != before {
            return Err(CheckpointError::CaptureUnstable.into());
        }
    }
    root.revalidate()?;
    Ok(FileImage::new(entries)?)
}

/// Keep physical evidence from the same file reads, never a later best-effort scan.
#[cfg(test)]
pub(super) fn observed(
    root: &Root,
    archive: &Archive,
    paths: &[CheckpointPath],
    ignore: IgnoreFingerprint,
    cancellation: &CancellationToken,
) -> Result<ObservedSnapshot, Error> {
    observe_to(
        root,
        Destination::Archive(archive),
        paths,
        ignore,
        cancellation,
    )
}
pub(super) fn observe_to(
    root: &Root,
    destination: Destination<'_>,
    paths: &[CheckpointPath],
    ignore: IgnoreFingerprint,
    cancellation: &CancellationToken,
) -> Result<ObservedSnapshot, Error> {
    let before = paths
        .iter()
        .map(|path| {
            Ok(ObservedEntry {
                path: path.clone(),
                presence: root.observe(path)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let image = image_to(root, destination, paths, cancellation)?;
    for entry in &before {
        if root.observe(&entry.path)? != entry.presence {
            return Err(CheckpointError::CaptureUnstable.into());
        }
    }
    let evidence = SnapshotEvidence::new(root.physical(), root.fingerprint(), ignore, before)?;
    Ok(ObservedSnapshot::new(image, evidence)?)
}
