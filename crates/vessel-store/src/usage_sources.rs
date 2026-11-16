//! Source grants persist identities/revisions only; atomic owner-only writes match usage.lookup.
use crate::{DataDir, StoreError};
use std::{
    fs::File,
    io::{self, Read},
};
use vessel_proto::usage_reports::ReportSourceGrants;
const MAX_SOURCE_POLICY_BYTES: u64 = 16_384;
impl DataDir {
    /// Load explicit source consent without discovering any login; missing means every source Off.
    /// # Errors
    /// Non-regular, unreadable, oversized, malformed or duplicate source grants fail closed.
    pub fn read_usage_sources(&self) -> Result<ReportSourceGrants, StoreError> {
        let path = self.root().join("usage-sources.json");
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(ReportSourceGrants::default());
            }
            Err(error) => return Err(StoreError::io(&path)(error)),
        };
        if !file.metadata().map_err(StoreError::io(&path))?.is_file() {
            return Err(StoreError::io(&path)(io::Error::other(
                "source consent must be a regular file",
            )));
        }
        let mut bytes = Vec::new();
        let _read = file
            .take(MAX_SOURCE_POLICY_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(StoreError::io(&path))?;
        if u64::try_from(bytes.len()).map_or(true, |size| size > MAX_SOURCE_POLICY_BYTES) {
            return Err(StoreError::io(&path)(io::Error::other(
                "source consent exceeds its supported size",
            )));
        }
        serde_json::from_slice(&bytes)
            .map_err(|cause| StoreError::io(&path)(io::Error::other(cause)))
    }
    /// Save only explicit source consent, never inputs, local paths, tokens or report snapshots.
    /// # Errors
    /// The owner-only atomic replacement could not complete or the complete policy is oversized.
    pub fn write_usage_sources(&self, grants: &ReportSourceGrants) -> Result<(), StoreError> {
        let path = self.root().join("usage-sources.json");
        let bytes = serde_json::to_vec(grants)
            .map_err(|cause| StoreError::io(&path)(io::Error::other(cause)))?;
        if u64::try_from(bytes.len()).map_or(true, |size| size > MAX_SOURCE_POLICY_BYTES) {
            return Err(StoreError::io(&path)(io::Error::other(
                "source consent exceeds its supported size",
            )));
        }
        crate::files::replace(&path, &bytes).map_err(StoreError::io(&path))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::usage_reports::{UsageDisclosureVersion, UsageSourceGrant};
    #[test]
    fn absent_and_invalid_source_policies_never_inherit_the_legacy_opt_in() {
        let root = tempfile::tempdir().unwrap();
        let dir = DataDir::new(root.path());
        dir.write_usage_lookup(vessel_proto::limits::ExternalLookup::On)
            .unwrap();
        assert!(dir.read_usage_sources().unwrap().is_empty());
        let grants: ReportSourceGrants = vec![UsageSourceGrant {
            id: "grok".parse().unwrap(),
            version: UsageDisclosureVersion::INITIAL,
        }]
        .try_into()
        .unwrap();
        dir.write_usage_sources(&grants).unwrap();
        assert_eq!(dir.read_usage_sources().unwrap(), grants);
        let path = root.path().join("usage-sources.json");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        for bytes in [
            br#"[{"id":"grok","version":0}]"#.as_slice(),
            br#"[{"id":"grok","version":1},{"id":"grok","version":1}]"#.as_slice(),
            br#"[{"id":"grok","version":1,"token":"synthetic"}]"#.as_slice(),
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(dir.read_usage_sources().is_err());
        }
        std::fs::write(
            &path,
            vec![b' '; usize::try_from(MAX_SOURCE_POLICY_BYTES + 1).unwrap()],
        )
        .unwrap();
        assert!(dir.read_usage_sources().is_err());
        dir.write_usage_sources(&ReportSourceGrants::default())
            .unwrap();
        assert!(dir.read_usage_sources().unwrap().is_empty());
    }
}
