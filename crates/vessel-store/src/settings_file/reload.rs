//! Explicit reload reads a bounded snapshot and refuses unknown/ignored data.
//! It never sets a file aside or writes it; startup retains its existing policy.

use std::fs::File;
use std::io::{self, Read as _};

use vessel_proto::settings::{InvalidSettingInput, SetNodeSettings};

use crate::{DataDir, StoreError};

/// Explicit application preserves the difference between bad data and failed I/O.
#[derive(Debug, thiserror::Error)]
pub enum ReloadSettingsError {
    /// No file contents are present in the typed input error or its sources.
    #[error("saved settings cannot be applied")]
    Input(#[from] InvalidSettingInput),
    /// Retain the actual filesystem failure rather than substituting defaults.
    #[error("saved settings cannot be read")]
    Read(#[source] StoreError),
}

impl DataDir {
    /// Read a single validated reload snapshot, before the node applies anything.
    ///
    /// # Errors
    /// The file cannot be read, exceeds the shared byte budget or contains data
    /// this build cannot faithfully represent. A missing file removes overrides.
    pub fn read_settings_for_reload(&self) -> Result<Option<SetNodeSettings>, ReloadSettingsError> {
        let path = self.settings_path();
        let fail = |error| ReloadSettingsError::Read(StoreError::io(&path)(error));
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(fail(error)),
        };
        let limit = u64::try_from(SetNodeSettings::MAX_JSON_BYTES)
            .map_err(io::Error::other)
            .map_err(fail)?;
        let mut bytes = Vec::new();
        // One extra byte detects overflow without allocating the rest of a file.
        let _bytes_read = file
            .take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(fail)?;
        SetNodeSettings::from_json(&bytes)
            .map(Some)
            .map_err(ReloadSettingsError::Input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_reload_preserves_invalid_and_excess_files_without_applying_them() {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        assert_eq!(data.read_settings_for_reload().unwrap(), None);
        let path = data.settings_path();
        let valid = br#"{"park_after":{"kind":"never"}}"#;
        std::fs::write(&path, valid).unwrap();
        assert!(
            data.read_settings_for_reload()
                .unwrap()
                .unwrap()
                .park_after
                .is_some()
        );
        for bytes in [
            b"{".as_slice(),
            br#"{"future":"private-reload-canary"}"#,
            br#"{"park_after":{"kind":"never"},"drivers":{"codex":"private-reload-canary"}}"#,
        ] {
            std::fs::write(&path, bytes).unwrap();
            let error = data.read_settings_for_reload().unwrap_err();
            assert!(!format!("{error:?}").contains("private-reload-canary"));
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        let bytes = vec![b'!'; SetNodeSettings::MAX_JSON_BYTES.saturating_add(1)];
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(
            data.read_settings_for_reload(),
            Err(ReloadSettingsError::Input(InvalidSettingInput::TooLong))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
