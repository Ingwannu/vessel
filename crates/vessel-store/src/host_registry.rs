//! An owner-only atomic directory snapshot contains references and annotations, no keys.
//! The node's single registry actor owns read/modify/write sequencing across cancellation.

use crate::{DataDir, StoreError};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, Read},
};
use vessel_proto::host_registry::{HostRegistration, MAX_HOST_REGISTRATIONS};

/// Whole-file admission keeps corrupt or hostile registry files from exhausting memory.
const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
const FILE: &str = "host-registry.json";

/// A validated snapshot cannot contain duplicate identities or an unbounded directory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DecodedRegistry", into = "DecodedRegistry")]
pub struct HostRegistryFile(Vec<HostRegistration>);
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecodedRegistry {
    entries: Vec<HostRegistration>,
}
impl TryFrom<DecodedRegistry> for HostRegistryFile {
    type Error = RegistryFileError;
    fn try_from(decoded: DecodedRegistry) -> Result<Self, Self::Error> {
        Self::new(decoded.entries)
    }
}
impl From<HostRegistryFile> for DecodedRegistry {
    fn from(file: HostRegistryFile) -> Self {
        Self { entries: file.0 }
    }
}
impl HostRegistryFile {
    /// Validate a candidate before committing it atomically.
    /// # Errors
    /// Duplicate IDs or too many registrations are refused as a whole.
    pub fn new(entries: Vec<HostRegistration>) -> Result<Self, RegistryFileError> {
        if entries.len() > MAX_HOST_REGISTRATIONS {
            return Err(RegistryFileError::TooMany);
        }
        let mut ids = BTreeSet::new();
        if entries
            .iter()
            .any(|entry| !ids.insert(entry.registration_id))
        {
            return Err(RegistryFileError::Duplicate);
        }
        Ok(Self(entries))
    }
    /// Move a validated candidate into the actor after its atomic persistence.
    #[must_use]
    pub fn into_entries(self) -> Vec<HostRegistration> {
        self.0
    }

    /// Borrow records without opening an unchecked mutation path.
    #[must_use]
    pub fn entries(&self) -> &[HostRegistration] {
        &self.0
    }
}
/// File errors retain I/O/decoding causes and never print metadata contents.
#[derive(Debug, thiserror::Error)]
pub enum RegistryFileError {
    /// A corrupt file must not be read as an empty directory.
    #[error("host registry could not be decoded")]
    Decode(#[source] serde_json::Error),
    /// Atomic persistence or a bounded read failed.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Admission cap is part of the public reference contract.
    #[error("host registry exceeds {MAX_HOST_REGISTRATIONS} registrations")]
    TooMany,
    /// A duplicate key would make an update/remove ambiguous.
    #[error("host registry contains a duplicate registration ID")]
    Duplicate,
    /// Oversized files are rejected before decoding.
    #[error("host registry exceeds its file byte budget")]
    TooLarge,
    /// Unexpected filesystem object never becomes registry state.
    #[error("host registry must be a regular file, not a symbolic link")]
    NotRegular,
}
impl DataDir {
    /// Load one bounded snapshot; missing means a new empty directory.
    /// # Errors
    /// Wrong object type, oversized input or malformed snapshots are refused.
    pub fn read_host_registry(&self) -> Result<HostRegistryFile, RegistryFileError> {
        let path = self.root().join(FILE);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(HostRegistryFile::default());
            }
            Err(error) => return Err(StoreError::io(&path)(error).into()),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(RegistryFileError::NotRegular);
        }
        let file = File::open(&path).map_err(StoreError::io(&path))?;
        let limit = u64::try_from(MAX_FILE_BYTES.saturating_add(1))
            .map_err(|error| StoreError::io(&path)(io::Error::other(error)))?;
        let mut bytes = Vec::new();
        let _read = file
            .take(limit)
            .read_to_end(&mut bytes)
            .map_err(StoreError::io(&path))?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(RegistryFileError::TooLarge);
        }
        serde_json::from_slice(&bytes).map_err(RegistryFileError::Decode)
    }
    /// Replace the snapshot with the existing store's 0600 atomic-file writer.
    /// # Errors
    /// Refuse before replacement on serialization/admission failure; I/O keeps its cause.
    pub fn write_host_registry(
        &self,
        snapshot: &HostRegistryFile,
    ) -> Result<(), RegistryFileError> {
        let path = self.root().join(FILE);
        let bytes = serde_json::to_vec(snapshot).map_err(RegistryFileError::Decode)?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(RegistryFileError::TooLarge);
        }
        crate::files::replace(&path, &bytes).map_err(StoreError::io(&path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{NodeId, host_registry::*};
    fn record() -> HostRegistration {
        HostRegistration {
            registration_id: HostRegistrationId::random(),
            source: HostSource::Owner,
            destination: HostDestination::Fleet {
                node: NodeId::random(),
            },
            label: Some("build".parse().unwrap()),
            metadata: HostMetadata::default(),
            enabled: HostEnabled::Enabled,
            revision: HostRevision::default(),
        }
    }
    #[test]
    fn registry_round_trip_is_private_and_duplicate_or_corrupt_files_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let dir = DataDir::new(root.path());
        let entry = record();
        let snapshot = HostRegistryFile::new(vec![entry.clone()]).unwrap();
        assert!(dir.read_host_registry().unwrap().entries().is_empty());
        assert!(HostRegistryFile::new(vec![entry.clone(), entry]).is_err());
        dir.write_host_registry(&snapshot).unwrap();
        assert_eq!(dir.read_host_registry().unwrap(), snapshot);
        let path = root.path().join(FILE);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::write(&path, b"{invalid").unwrap();
        assert!(dir.read_host_registry().is_err());
        dir.write_host_registry(&snapshot).unwrap();
        assert_eq!(dir.read_host_registry().unwrap(), snapshot);
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_read_or_written_through() {
        let root = tempfile::tempdir().unwrap();
        let dir = DataDir::new(root.path());
        let other = root.path().join("other");
        std::fs::write(&other, b"unchanged").unwrap();
        std::os::unix::fs::symlink(&other, root.path().join(FILE)).unwrap();
        assert!(matches!(
            dir.read_host_registry(),
            Err(RegistryFileError::NotRegular)
        ));
        dir.write_host_registry(&HostRegistryFile::default())
            .unwrap();
        assert_eq!(std::fs::read(&other).unwrap(), b"unchanged");
        assert!(
            !std::fs::symlink_metadata(root.path().join(FILE))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}
