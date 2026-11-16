//! Physical admission identity; no checkpoint calls, events or restore authority.
use crate::BlobHash;
use serde::{Deserialize, Serialize};
/// An opened directory's device/inode key, independent of logical workspace ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PhysicalRoot(BlobHash);
impl PhysicalRoot {
    /// Construct only from observations made at the filesystem edge.
    #[must_use]
    pub const fn new(hash: BlobHash) -> Self {
        Self(hash)
    }
    /// Compare or persist a key without exposing raw filesystem metadata.
    #[must_use]
    pub const fn digest(self) -> BlobHash {
        self.0
    }
}
