//! Preview derives the same path operations that effective-image verification uses.
use super::{CheckpointPath, EffectiveDesiredImage, FileImage, FileState};
use serde::{Deserialize, Serialize};

/// Confirmation describes filesystem effects, never agent tool-call counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "CheckpointChangeKind"))]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// Create a currently absent regular file.
    Create,
    /// Replace existing raw content.
    Replace,
    /// Remove a later-created covered file.
    Delete,
    /// Content is equal; executable policy changes metadata.
    Permissions,
}
/// One approved path effect in deterministic sorted order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "CheckpointChange"))]
pub struct Change {
    /// The validated root-relative path.
    pub path: CheckpointPath,
    /// Destructive effects are visible before admission.
    pub kind: ChangeKind,
    /// Covered byte count before the effect; absent means the path does not exist.
    pub before: Option<super::CheckpointFileBytes>,
    /// Covered byte count after the effect; absent means removal.
    pub after: Option<super::CheckpointFileBytes>,
}
impl EffectiveDesiredImage {
    /// Only differences from this frozen target appear in a confirmation.
    #[must_use]
    pub fn changes(&self, current: &FileImage) -> Vec<Change> {
        self.image()
            .entries()
            .iter()
            .filter_map(|desired| {
                let kind = match (&desired.state, current.get(&desired.path)) {
                    (FileState::Absent, None | Some(FileState::Absent)) => None,
                    (FileState::Absent, Some(FileState::Present { .. })) => {
                        Some(ChangeKind::Delete)
                    }
                    (FileState::Present { .. }, None | Some(FileState::Absent)) => {
                        Some(ChangeKind::Create)
                    }
                    (
                        FileState::Present {
                            digest,
                            bytes,
                            metadata,
                        },
                        Some(FileState::Present {
                            digest: old_digest,
                            bytes: old_bytes,
                            metadata: old_metadata,
                        }),
                    ) => {
                        if digest != old_digest || bytes != old_bytes {
                            Some(ChangeKind::Replace)
                        } else if metadata != old_metadata {
                            Some(ChangeKind::Permissions)
                        } else {
                            None
                        }
                    }
                };
                kind.map(|kind| Change {
                    path: desired.path.clone(),
                    kind,
                    before: size(current.get(&desired.path)),
                    after: size(Some(&desired.state)),
                })
            })
            .collect()
    }
}
fn size(state: Option<&FileState>) -> Option<super::CheckpointFileBytes> {
    match state {
        Some(FileState::Present { bytes, .. }) => Some(*bytes),
        None | Some(FileState::Absent) => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BlobHash,
        checkpoint::{
            CheckpointFileBytes, CheckpointPath, Entry, FileMetadata, GroupId, OrdinaryMode,
            OwnerId,
        },
    };
    fn metadata(mode: u32) -> FileMetadata {
        FileMetadata {
            owner: OwnerId::new(1000),
            group: GroupId::new(1000),
            mode: OrdinaryMode::new(mode).unwrap(),
        }
    }
    fn file(mode: u32, content: u8) -> FileImage {
        FileImage::new(vec![Entry {
            path: CheckpointPath::try_from("file".parse::<crate::workspace::RelPath>().unwrap())
                .unwrap(),
            state: FileState::Present {
                digest: BlobHash::from_bytes([content; 32]),
                bytes: CheckpointFileBytes::new(1).unwrap(),
                metadata: metadata(mode),
            },
        }])
        .unwrap()
    }
    #[test]
    fn confirmation_uses_effective_permissions_not_raw_capture_permissions() {
        let captured = file(0o644, 1);
        let current = file(0o600, 1);
        assert!(
            captured
                .effective(&current, metadata(0o600))
                .unwrap()
                .changes(&current)
                .is_empty()
        );
        let changed = file(0o644, 2)
            .effective(&current, metadata(0o600))
            .unwrap()
            .changes(&current);
        assert_eq!(changed.first().unwrap().kind, ChangeKind::Replace);
        let executable = file(0o755, 1)
            .effective(&current, metadata(0o600))
            .unwrap()
            .changes(&current);
        assert_eq!(executable.first().unwrap().kind, ChangeKind::Permissions);
    }
    #[test]
    fn create_and_later_file_deletion_are_explicit_not_rename_guesses() {
        let empty = FileImage::new(Vec::new()).unwrap();
        let captured = file(0o644, 1);
        assert_eq!(
            captured
                .effective(&empty, metadata(0o600))
                .unwrap()
                .changes(&empty)
                .first()
                .unwrap()
                .kind,
            ChangeKind::Create
        );
        assert_eq!(
            empty
                .effective(&captured, metadata(0o600))
                .unwrap()
                .changes(&captured)
                .first()
                .unwrap()
                .kind,
            ChangeKind::Delete
        );
    }
}
