//! Private durable inode evidence shared by workspace I/O and store journals.
//! These are storage values, never client-supplied filesystem authority or RPCs.
use super::{CheckpointError, CheckpointPath, ObjectFingerprint, PathCount};
use serde::{Deserialize, Serialize};

/// Stable original/written objects for one approved path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedPath {
    /// Approved root-relative path.
    pub path: CheckpointPath,
    /// Original inode identity; absent when the path did not exist.
    pub before: Option<ObjectFingerprint>,
    /// Staged inode that this restore installs; absent for deletion.
    pub desired: Option<ObjectFingerprint>,
}

/// Bounded canonical evidence, durably committed before the first project mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<StagedPath>", into = "Vec<StagedPath>")]
pub struct StagedPaths(Vec<StagedPath>);

impl StagedPaths {
    /// Bound and sort the exact approved mutation evidence.
    /// # Errors
    /// Too many paths, duplicate names, or an impossible absent-to-absent change.
    pub fn new(mut paths: Vec<StagedPath>) -> Result<Self, CheckpointError> {
        let _count = PathCount::new(paths.len())?;
        paths.sort_by(|left, right| left.path.cmp(&right.path));
        if paths
            .windows(2)
            .any(|pair| matches!(pair, [left,right] if left.path==right.path))
        {
            return Err(CheckpointError::DuplicatePath);
        }
        if paths
            .iter()
            .any(|path| path.before.is_none() && path.desired.is_none())
        {
            return Err(CheckpointError::StalePreview);
        }
        Ok(Self(paths))
    }
    /// Canonical path order also defines the private stage slot index.
    #[must_use]
    pub fn entries(&self) -> &[StagedPath] {
        &self.0
    }
}
impl TryFrom<Vec<StagedPath>> for StagedPaths {
    type Error = CheckpointError;
    fn try_from(paths: Vec<StagedPath>) -> Result<Self, Self::Error> {
        Self::new(paths)
    }
}
impl From<StagedPaths> for Vec<StagedPath> {
    fn from(paths: StagedPaths) -> Self {
        paths.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkpoint_staged_evidence_rejects_duplicate_and_impossible_mutations() {
        let row = StagedPath {
            path: serde_json::from_str("\"file\"").unwrap(),
            before: None,
            desired: Some(ObjectFingerprint::new(crate::BlobHash::from_bytes([1; 32]))),
        };
        assert!(matches!(
            StagedPaths::new(vec![row.clone(), row.clone()]),
            Err(CheckpointError::DuplicatePath)
        ));
        assert!(
            StagedPaths::new(vec![StagedPath {
                desired: None,
                ..row.clone()
            }])
            .is_err()
        );
        let staged = StagedPaths::new(vec![row]).unwrap();
        assert_eq!(
            serde_json::from_str::<StagedPaths>(&serde_json::to_string(&staged).unwrap()).unwrap(),
            staged
        );
    }
}
