//! Checkpoint authority follows the actual effective service UID, never a name.
use super::{Error, rooted::refuse_attributes};
use std::{
    fs::{File, Metadata},
    os::unix::fs::MetadataExt as _,
};
use vessel_proto::checkpoint::CheckpointError;

const PERMISSION_BITS: u32 = 0o7777;
const PRIVATE_DIRECTORY: u32 = 0o700;

pub(super) fn owned(metadata: &Metadata) -> Result<(), Error> {
    if metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(CheckpointError::UnsupportedMetadata.into());
    }
    Ok(())
}

pub(super) fn private_directory(file: &File) -> Result<(), Error> {
    let metadata = file.metadata()?;
    owned(&metadata)?;
    if !metadata.is_dir() || metadata.mode() & PERMISSION_BITS != PRIVATE_DIRECTORY {
        return Err(CheckpointError::UnsupportedMetadata.into());
    }
    refuse_attributes(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt as _};
    #[test]
    fn checkpoint_service_uid_owns_private_storage_and_refuses_foreign_root_metadata() {
        assert_ne!(
            rustix::process::geteuid().as_raw(),
            0,
            "run permission fixtures as the actual non-root build account"
        );
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(PRIVATE_DIRECTORY)).unwrap();
        let file = File::open(dir.path()).unwrap();
        private_directory(&file).unwrap();
        let foreign = fs::metadata("/").unwrap();
        assert_ne!(foreign.uid(), rustix::process::geteuid().as_raw());
        assert!(matches!(
            owned(&foreign),
            Err(Error::Refused(CheckpointError::UnsupportedMetadata))
        ));
        assert!(matches!(
            super::super::rooted::Root::open(std::path::Path::new("/")),
            Err(Error::Refused(CheckpointError::UnsupportedMetadata))
        ));
    }
    #[test]
    fn checkpoint_private_storage_refuses_access_and_special_mode_bits() {
        let dir = tempfile::tempdir().unwrap();
        for mode in [0o750, 0o2700, 0o1700] {
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(mode)).unwrap();
            let file = File::open(dir.path()).unwrap();
            assert!(matches!(
                private_directory(&file),
                Err(Error::Refused(CheckpointError::UnsupportedMetadata))
            ));
        }
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(PRIVATE_DIRECTORY)).unwrap();
    }
}
