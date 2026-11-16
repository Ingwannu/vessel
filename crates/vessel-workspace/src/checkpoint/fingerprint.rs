//! Versioned canonical image digest avoids unstable JSON/map serialization details.
use sha2::{Digest as _, Sha256};
use vessel_proto::{
    BlobHash,
    checkpoint::{CheckpointError, FileImage, FileState},
};

pub(super) fn image(image: &FileImage) -> Result<BlobHash, CheckpointError> {
    let mut digest = Sha256::new();
    digest.update(b"vessel-checkpoint-image-v1\0");
    for entry in image.entries() {
        let path = entry.path.relative().as_str().as_bytes();
        digest.update(
            u64::try_from(path.len())
                .map_err(|_large| CheckpointError::InvalidPath)?
                .to_le_bytes(),
        );
        digest.update(path);
        match entry.state {
            FileState::Absent => digest.update(b"absent\0"),
            FileState::Present {
                digest: content,
                bytes,
                metadata,
            } => {
                digest.update(b"present\0");
                digest.update(content.as_bytes());
                digest.update(bytes.get().to_le_bytes());
                digest.update(metadata.owner.get().to_le_bytes());
                digest.update(metadata.group.get().to_le_bytes());
                digest.update(metadata.mode.get().to_le_bytes());
            }
        }
    }
    Ok(BlobHash::from_bytes(digest.finalize().into()))
}
#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::checkpoint::{
        CheckpointFileBytes, CheckpointPath, Entry, FileMetadata, GroupId, OrdinaryMode, OwnerId,
    };
    fn entry(path: &str, mode: u32) -> Entry {
        Entry {
            path: CheckpointPath::try_from(
                path.parse::<vessel_proto::workspace::RelPath>().unwrap(),
            )
            .unwrap(),
            state: FileState::Present {
                digest: BlobHash::from_bytes([1; 32]),
                bytes: CheckpointFileBytes::new(1).unwrap(),
                metadata: FileMetadata {
                    owner: OwnerId::new(1000),
                    group: GroupId::new(1000),
                    mode: OrdinaryMode::new(mode).unwrap(),
                },
            },
        }
    }
    #[test]
    fn fingerprint_is_order_independent_but_binds_path_and_effective_permissions() {
        let first = FileImage::new(vec![entry("b", 0o600), entry("a", 0o600)]).unwrap();
        let reordered = FileImage::new(vec![entry("a", 0o600), entry("b", 0o600)]).unwrap();
        let executable = FileImage::new(vec![entry("a", 0o700), entry("b", 0o600)]).unwrap();
        let renamed = FileImage::new(vec![entry("c", 0o600), entry("b", 0o600)]).unwrap();
        assert_eq!(image(&first).unwrap(), image(&reordered).unwrap());
        assert_ne!(image(&first).unwrap(), image(&executable).unwrap());
        assert_ne!(image(&first).unwrap(), image(&renamed).unwrap());
    }
}
