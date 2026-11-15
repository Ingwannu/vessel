//! Filesystem observations bind a preview without replaying captured inode metadata.
use super::{CheckpointError, CheckpointPath, FileImage, FileState, PathCount};
use crate::BlobHash;
use serde::{Deserialize, Serialize};

macro_rules! fingerprint {
    ($name:ident,$doc:literal) => {
        #[doc=$doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(BlobHash);
        impl $name {
            /// Accept a canonical SHA-256 digest computed at the filesystem edge.
            #[must_use]
            pub const fn new(hash: BlobHash) -> Self {
                Self(hash)
            }
            /// Compare or persist the fingerprint without exposing raw metadata.
            #[must_use]
            pub const fn digest(self) -> BlobHash {
                self.0
            }
        }
    };
}
fingerprint!(
    PhysicalRoot,
    "An FD-derived device/inode admission identity, independent of logical workspaces."
);
fingerprint!(
    RootFingerprint,
    "A physical root observation used to reject stale plans."
);
fingerprint!(
    IgnoreFingerprint,
    "A digest of every effective ignore-policy input."
);
fingerprint!(
    FileFingerprint,
    "A complete file observation, including timestamps and metadata."
);
fingerprint!(
    ObjectFingerprint,
    "A stable inode observation that survives our own renames."
);

/// Presence and object identity are distinct from an empty file's contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservedPresence {
    /// No entry existed at the observed path.
    Absent,
    /// A regular, safe file existed with these observations.
    Present {
        /// Changes of the original observation stale the confirmation.
        full: FileFingerprint,
        /// Per-path mutation and recovery retain the same original object.
        object: ObjectFingerprint,
    },
}
/// One root-relative observation; all names use the existing validated path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedEntry {
    /// Path within the registered managed worktree.
    pub path: CheckpointPath,
    /// No filesystem identifier is supplied by an RPC caller.
    pub presence: ObservedPresence,
}
/// Private persistent evidence accompanies a content image, never its effective target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawEvidence", into = "RawEvidence")]
pub struct SnapshotEvidence {
    physical: PhysicalRoot,
    root: RootFingerprint,
    ignore: IgnoreFingerprint,
    entries: Vec<ObservedEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawEvidence {
    physical: PhysicalRoot,
    root: RootFingerprint,
    ignore: IgnoreFingerprint,
    entries: Vec<ObservedEntry>,
}
impl SnapshotEvidence {
    /// Bound and canonicalize observations before they become durable plan input.
    ///
    /// # Errors
    /// Too many paths or duplicate names refuse the whole evidence set.
    pub fn new(
        physical: PhysicalRoot,
        root: RootFingerprint,
        ignore: IgnoreFingerprint,
        mut entries: Vec<ObservedEntry>,
    ) -> Result<Self, CheckpointError> {
        let _count = PathCount::new(entries.len())?;
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        if entries
            .windows(2)
            .any(|pair| matches!(pair,[left,right] if left.path==right.path))
        {
            return Err(CheckpointError::DuplicatePath);
        }
        Ok(Self {
            physical,
            root,
            ignore,
            entries,
        })
    }
    /// Shared writer exclusion key; access-mode changes do not mint another lock.
    #[must_use]
    pub const fn physical(&self) -> PhysicalRoot {
        self.physical
    }
    /// Frozen physical root, not the logical workspace's identity.
    #[must_use]
    pub const fn root(&self) -> RootFingerprint {
        self.root
    }
    /// Frozen ignore rules; a changed policy cannot widen coverage silently.
    #[must_use]
    pub const fn ignore(&self) -> IgnoreFingerprint {
        self.ignore
    }
    /// Canonically sorted path observations.
    #[must_use]
    pub fn entries(&self) -> &[ObservedEntry] {
        &self.entries
    }
}
impl TryFrom<RawEvidence> for SnapshotEvidence {
    type Error = CheckpointError;
    fn try_from(raw: RawEvidence) -> Result<Self, Self::Error> {
        Self::new(raw.physical, raw.root, raw.ignore, raw.entries)
    }
}
impl From<SnapshotEvidence> for RawEvidence {
    fn from(value: SnapshotEvidence) -> Self {
        Self {
            physical: value.physical,
            root: value.root,
            ignore: value.ignore,
            entries: value.entries,
        }
    }
}
/// An image and exactly the observations that made it eligible for publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawSnapshot", into = "RawSnapshot")]
pub struct ObservedSnapshot {
    image: FileImage,
    evidence: SnapshotEvidence,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawSnapshot {
    image: FileImage,
    evidence: SnapshotEvidence,
}
impl ObservedSnapshot {
    /// Refuse omissions or contradictory presence before storing a ready candidate.
    ///
    /// # Errors
    /// `CaptureUnstable` when observations and content do not cover the same entries.
    pub fn new(image: FileImage, evidence: SnapshotEvidence) -> Result<Self, CheckpointError> {
        if image.entries().len() != evidence.entries.len()
            || image
                .entries()
                .iter()
                .zip(&evidence.entries)
                .any(|(content, observed)| {
                    content.path != observed.path
                        || !matches!(
                            (&content.state, observed.presence),
                            (FileState::Absent, ObservedPresence::Absent)
                                | (FileState::Present { .. }, ObservedPresence::Present { .. })
                        )
                })
        {
            return Err(CheckpointError::CaptureUnstable);
        }
        Ok(Self { image, evidence })
    }
    /// Logical contents and safe ordinary metadata.
    #[must_use]
    pub const fn image(&self) -> &FileImage {
        &self.image
    }
    /// Observations for stale admission; never substituted into desired permissions.
    #[must_use]
    pub const fn evidence(&self) -> &SnapshotEvidence {
        &self.evidence
    }
}
impl TryFrom<RawSnapshot> for ObservedSnapshot {
    type Error = CheckpointError;
    fn try_from(raw: RawSnapshot) -> Result<Self, Self::Error> {
        Self::new(raw.image, raw.evidence)
    }
}
impl From<ObservedSnapshot> for RawSnapshot {
    fn from(value: ObservedSnapshot) -> Self {
        Self {
            image: value.image,
            evidence: value.evidence,
        }
    }
}

#[cfg(test)]
mod tests;
