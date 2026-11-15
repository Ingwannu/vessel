//! Device numbers do not distinguish bind mounts on the same filesystem.
use crate::checkpoint::Error;
use rustix::fs::{AtFlags, StatxFlags};
use std::fs::File;
use vessel_proto::checkpoint::CheckpointError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::checkpoint) struct MountId(u64);
impl MountId {
    pub(in crate::checkpoint) const fn get(self) -> u64 {
        self.0
    }
    pub(in crate::checkpoint) fn of(file: &File) -> Result<Self, Error> {
        let found = rustix::fs::statx(file, "", AtFlags::EMPTY_PATH, StatxFlags::MNT_ID)
            .map_err(std::io::Error::from)?;
        Self::from_stat(found.stx_mask, found.stx_mnt_id)
    }
    fn from_stat(mask: u32, id: u64) -> Result<Self, Error> {
        if mask & StatxFlags::MNT_ID.bits() == 0 {
            return Err(CheckpointError::UnsupportedPlatform.into());
        }
        Ok(Self(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identical_device_information_cannot_substitute_for_a_mount_identity() {
        assert_ne!(
            MountId::from_stat(StatxFlags::MNT_ID.bits(), 1).unwrap(),
            MountId::from_stat(StatxFlags::MNT_ID.bits(), 2).unwrap()
        );
        assert!(matches!(
            MountId::from_stat(0, 1),
            Err(Error::Refused(CheckpointError::UnsupportedPlatform))
        ));
    }
    #[test]
    fn actual_fd_metadata_exposes_the_mount_without_reading_file_contents() {
        let temp = tempfile::tempdir().unwrap();
        let root = File::open(temp.path()).unwrap();
        std::fs::write(temp.path().join("file"), b"contents").unwrap();
        let file = File::open(temp.path().join("file")).unwrap();
        assert_eq!(MountId::of(&root).unwrap(), MountId::of(&file).unwrap());
    }
}
