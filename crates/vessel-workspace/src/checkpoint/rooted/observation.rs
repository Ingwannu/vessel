//! Opaque persisted hashes bind previews to physical observations, not just bytes.
use super::{Error, Identity, Root};
use sha2::{Digest as _, Sha256};
use vessel_proto::{
    BlobHash,
    checkpoint::{
        CheckpointPath, FileFingerprint, ObjectFingerprint, ObservedPresence, PhysicalRoot,
        RootFingerprint,
    },
};
impl Identity {
    fn object_digest(self) -> Sha256 {
        let mut digest = Sha256::new();
        digest.update(b"vessel-checkpoint-object-v1\0");
        digest.update(self.device.to_le_bytes());
        digest.update(self.inode.to_le_bytes());
        digest.update(self.owner.to_le_bytes());
        digest.update(self.group.to_le_bytes());
        digest.update(self.links.to_le_bytes());
        digest
    }
    pub(in crate::checkpoint) fn object_fingerprint(self) -> ObjectFingerprint {
        ObjectFingerprint::new(BlobHash::from_bytes(self.object_digest().finalize().into()))
    }
    pub(in crate::checkpoint) fn observation(self) -> ObservedPresence {
        let object = self.object_fingerprint();
        let mut digest = self.object_digest();
        digest.update(b"full\0");
        digest.update(self.size.to_le_bytes());
        digest.update(self.mode.to_le_bytes());
        digest.update(self.modified.0.to_le_bytes());
        digest.update(self.modified.1.to_le_bytes());
        digest.update(self.changed.0.to_le_bytes());
        digest.update(self.changed.1.to_le_bytes());
        ObservedPresence::Present {
            full: FileFingerprint::new(BlobHash::from_bytes(digest.finalize().into())),
            object,
        }
    }
}
impl Root {
    pub(in crate::checkpoint) fn physical(&self) -> PhysicalRoot {
        self.physical
    }
    pub(in crate::checkpoint) fn fingerprint(&self) -> RootFingerprint {
        // Directory mtimes change during our own safe renames. Root authority is
        // its stable object, access policy and mount, never that mutable timestamp.
        let mut digest = self.identity.object_digest();
        digest.update(b"root\0");
        digest.update(self.identity.mode.to_le_bytes());
        digest.update(self.mount.get().to_le_bytes());
        RootFingerprint::new(BlobHash::from_bytes(digest.finalize().into()))
    }
    pub(in crate::checkpoint) fn observe(
        &self,
        path: &CheckpointPath,
    ) -> Result<ObservedPresence, Error> {
        Ok(self
            .entry_identity(path)?
            .map_or(ObservedPresence::Absent, Identity::observation))
    }
}
