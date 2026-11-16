//! Immutable bounded images and the single effective target used throughout restore.

use super::{
    CheckpointError, CheckpointFileBytes, CheckpointPath, FileMetadata, ImageBytes, PathCount,
};
use crate::BlobHash;
use serde::{Deserialize, Serialize};

/// Missing files are evidence, not zero-byte contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FileState {
    /// The path was absent at this boundary.
    Absent,
    /// Raw bytes in immutable private blob storage.
    Present {
        /// Digest verified while streaming.
        digest: BlobHash,
        /// Bounded raw size.
        bytes: CheckpointFileBytes,
        /// Capture evidence or effective desired metadata.
        metadata: FileMetadata,
    },
}
/// A named entry whose path can never be a root or Git administrative directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Portable nonroot path under the registered worktree.
    pub path: CheckpointPath,
    /// Captured presence/content/ordinary metadata.
    pub state: FileState,
}
/// A sorted unique complete image: validated even when read from stored JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Entry>", into = "Vec<Entry>")]
pub struct FileImage(Vec<Entry>);
impl FileImage {
    /// Bound count, total bytes and duplicate names before accepting an image.
    ///
    /// # Errors
    /// Oversized images/counts or repeated paths refuse the entire image.
    pub fn new(mut entries: Vec<Entry>) -> Result<Self, CheckpointError> {
        let _count = PathCount::new(entries.len())?;
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        let mut previous = None;
        let mut total = 0_u64;
        for entry in &entries {
            if previous == Some(&entry.path) {
                return Err(CheckpointError::DuplicatePath);
            }
            previous = Some(&entry.path);
            if let FileState::Present { bytes, .. } = entry.state {
                total = total
                    .checked_add(bytes.get())
                    .ok_or(CheckpointError::ImageTooLarge)?;
            }
        }
        let _bytes = ImageBytes::new(total)?;
        Ok(Self(entries))
    }
    /// Sorted entries support deterministic plans and fingerprints.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.0
    }
    /// Locate existing content without assuming identical file enumeration.
    #[must_use]
    pub fn get(&self, path: &CheckpointPath) -> Option<&FileState> {
        self.0
            .binary_search_by(|entry| entry.path.cmp(path))
            .ok()
            .and_then(|index| self.0.get(index))
            .map(|entry| &entry.state)
    }
    /// Freeze checkpoint contents with the metadata policy applied to current files.
    ///
    /// # Errors
    /// The union of captured and current paths exceeds the image bounds.
    pub fn effective(
        &self,
        current: &Self,
        account: FileMetadata,
    ) -> Result<EffectiveDesiredImage, CheckpointError> {
        let mut entries = self.0.clone();
        for entry in &mut entries {
            if let FileState::Present { metadata, .. } = &mut entry.state {
                let existing = match current.get(&entry.path) {
                    Some(FileState::Present { metadata, .. }) => Some(*metadata),
                    Some(FileState::Absent) | None => None,
                };
                *metadata = metadata.effective(existing, account);
            }
        }
        // Later-created covered files are approved explicit deletions.
        entries.extend(
            current
                .0
                .iter()
                .filter(|entry| self.get(&entry.path).is_none())
                .map(|entry| Entry {
                    path: entry.path.clone(),
                    state: FileState::Absent,
                }),
        );
        Ok(EffectiveDesiredImage(Self::new(entries)?))
    }
}

/// Frozen post-policy target, distinct from the raw captured evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EffectiveDesiredImage(FileImage);
impl EffectiveDesiredImage {
    /// Immutable entries are shared by preview, journal and verification.
    #[must_use]
    pub const fn image(&self) -> &FileImage {
        &self.0
    }
    /// Missing paths satisfy explicit absence, never empty-file contents.
    #[must_use]
    pub fn matches(&self, current: &FileImage) -> bool {
        self.0
            .entries()
            .iter()
            .all(|entry| match (&entry.state, current.get(&entry.path)) {
                (FileState::Absent, None | Some(FileState::Absent)) => true,
                (state, Some(actual)) => state == actual,
                (FileState::Present { .. }, None) => false,
            })
            && current
                .entries()
                .iter()
                .all(|entry| self.0.get(&entry.path).is_some() || entry.state == FileState::Absent)
    }
}
impl TryFrom<Vec<Entry>> for FileImage {
    type Error = CheckpointError;
    fn try_from(entries: Vec<Entry>) -> Result<Self, Self::Error> {
        Self::new(entries)
    }
}
impl From<FileImage> for Vec<Entry> {
    fn from(image: FileImage) -> Self {
        image.0
    }
}

#[cfg(test)]
mod tests;
