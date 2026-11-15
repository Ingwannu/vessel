//! The explicitly saved credential-use choice, separate from snapshots.
//! A short owner-only atomic file avoids mixing private account state into
//! sync bundles. The node's actor owns write/apply across RPC cancellation.
use crate::{DataDir, StoreError};
use std::{
    fs::File,
    io::{self, Read},
};
use vessel_proto::limits::ExternalLookup;
impl DataDir {
    /// The saved local lookup choice; absent means Off. Reads at most 4KiB.
    /// # Errors
    /// An unreadable, oversized or malformed file is refused; callers fail off.
    pub fn read_usage_lookup(&self) -> Result<ExternalLookup, StoreError> {
        let path = self.root().join("usage-lookup.json");
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(ExternalLookup::Off);
            }
            Err(error) => return Err(StoreError::io(&path)(error)),
        };
        if !file.metadata().map_err(StoreError::io(&path))?.is_file() {
            return Err(StoreError::io(&path)(io::Error::other(
                "usage lookup policy must be a regular file",
            )));
        }
        let mut bytes = Vec::new();
        let _read = file
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(StoreError::io(&path))?;
        if bytes.len() > 4096 {
            return Err(StoreError::io(&path)(io::Error::other(
                "usage lookup policy is too large",
            )));
        }
        serde_json::from_slice(&bytes)
            .map_err(|source| StoreError::io(&path)(io::Error::other(source)))
    }
    /// Persist only the credential-use decision, never a login or snapshot.
    /// # Errors
    /// Atomic replacement could not complete in the node's data directory.
    pub fn write_usage_lookup(&self, policy: ExternalLookup) -> Result<(), StoreError> {
        let path = self.root().join("usage-lookup.json");
        let bytes = serde_json::to_vec(&policy)
            .map_err(|source| StoreError::io(&path)(io::Error::other(source)))?;
        crate::files::replace(&path, &bytes).map_err(StoreError::io(&path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_choice_is_saved_and_invalid_policy_never_becomes_opt_in() {
        let root = tempfile::tempdir().unwrap();
        let dir = DataDir::new(root.path());
        assert_eq!(dir.read_usage_lookup().unwrap(), ExternalLookup::Off);
        dir.write_usage_lookup(ExternalLookup::On).unwrap();
        let path = root.path().join("usage-lookup.json");
        assert_eq!(std::fs::read(&path).unwrap(), br#""on""#);
        assert_eq!(dir.read_usage_lookup().unwrap(), ExternalLookup::On);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        for body in [
            br#"{"policy":"on","token":"synthetic-only"}"#.as_slice(),
            br#""future""#.as_slice(),
        ] {
            std::fs::write(&path, body).unwrap();
            assert!(dir.read_usage_lookup().is_err());
        }
        std::fs::write(&path, vec![b' '; 4097]).unwrap();
        assert!(dir.read_usage_lookup().is_err());
        dir.write_usage_lookup(ExternalLookup::Off).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), br#""off""#);
    }
}
