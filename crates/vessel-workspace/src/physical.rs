//! FD-derived identity for the node's physical-root admission, not a workspace id.
mod ancestry;
pub use ancestry::Ancestry;
use sha2::{Digest as _, Sha256};
use std::{
    fs::{File, Metadata, OpenOptions},
    io,
    os::unix::fs::{MetadataExt as _, OpenOptionsExt as _},
    path::{Path, PathBuf},
};
use vessel_proto::{BlobHash, checkpoint::PhysicalRoot};

/// An open directory pins its inode while the node acquires and holds admission.
#[derive(Debug)]
pub struct Directory {
    file: File,
    path: PathBuf,
    identity: PhysicalRoot,
}
impl Directory {
    /// Resolve admission from the opened directory, refusing a final symlink.
    ///
    /// # Errors
    /// Missing, relative, non-directory or inaccessible paths preserve the I/O source.
    pub fn open(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "root must be absolute",
            ));
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(
                (nix::fcntl::OFlag::O_DIRECTORY
                    | nix::fcntl::OFlag::O_NOFOLLOW
                    | nix::fcntl::OFlag::O_NONBLOCK
                    | nix::fcntl::OFlag::O_CLOEXEC)
                    .bits(),
            )
            .open(path)?;
        let identity = key(&file.metadata()?);
        Ok(Self {
            file,
            path: path.to_path_buf(),
            identity,
        })
    }
    /// Aliases and access-mode changes never produce another writer lock.
    #[must_use]
    pub const fn identity(&self) -> PhysicalRoot {
        self.identity
    }
    /// Recheck the name against the retained FD before and after physical work.
    ///
    /// # Errors
    /// A replaced/moved root refuses rather than authorizing a different inode.
    pub fn revalidate(&self) -> io::Result<()> {
        let current = Self::open(&self.path)?;
        if current.identity != self.identity || key(&self.file.metadata()?) != self.identity {
            return Err(io::Error::other("physical root changed"));
        }
        Ok(())
    }
}
pub(crate) fn key(metadata: &Metadata) -> PhysicalRoot {
    let mut digest = Sha256::new();
    digest.update(b"vessel-physical-root-v1\0");
    digest.update(metadata.dev().to_le_bytes());
    digest.update(metadata.ino().to_le_bytes());
    PhysicalRoot::new(BlobHash::from_bytes(digest.finalize().into()))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    #[test]
    fn checkpoint_physical_admission_pins_the_open_object_and_detects_name_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        std::fs::create_dir(&root).unwrap();
        let directory = Directory::open(&root).unwrap();
        let identity = directory.identity();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(Directory::open(&root).unwrap().identity(), identity);
        directory.revalidate().unwrap();
        std::fs::rename(&root, temp.path().join("retained")).unwrap();
        std::fs::create_dir(&root).unwrap();
        assert_ne!(Directory::open(&root).unwrap().identity(), identity);
        assert!(directory.revalidate().is_err());
        let alias = temp.path().join("alias");
        symlink(&root, &alias).unwrap();
        assert!(Directory::open(&alias).is_err());
    }
}
