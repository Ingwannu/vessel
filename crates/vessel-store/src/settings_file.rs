//! `settings.json` in the data directory (protocol 12, ADR 0038): the node
//! settings a person changed through `node.set_settings`.
//!
//! [Decision Log]
//! - Purpose: a setting changed from the app must survive a restart, and a
//!   service started with `vesseld --park-after` must not undo it at its
//!   next start.
//! - Choice: only what a person changed is kept (every field optional),
//!   laid over the flags' values when the node starts; a setting nobody
//!   changed keeps following its flag. The same type as the wire's
//!   change (`SetNodeSettings`), so a later setting is one more optional
//!   field here too.
//! - A file that cannot be read or does not parse is not fatal: the node
//!   logs it and starts from its flags, and the next change replaces it.
//!   It is written owner-only and atomically, as `agents.toml` is, so a
//!   reader sees the old file or the new one.

use std::io;
use std::path::PathBuf;

use vessel_proto::settings::SetNodeSettings;

use crate::data_dir::DataDir;
use crate::error::StoreError;
use crate::files;

const SETTINGS_FILE: &str = "settings.json";

/// Why `settings.json` was not read.
#[derive(Debug, thiserror::Error)]
pub enum SettingsFileError {
    /// It could not be read.
    #[error(transparent)]
    Read(#[from] StoreError),
    /// It is not the settings' JSON.
    #[error("{}: {source}", path.display())]
    Parse {
        /// The file.
        path: PathBuf,
        /// What did not parse.
        #[source]
        source: serde_json::Error,
    },
}

impl DataDir {
    /// The node settings a person changed (protocol 12).
    #[must_use]
    pub fn settings_path(&self) -> PathBuf {
        self.root().join(SETTINGS_FILE)
    }

    /// The saved settings, or `None` when none were ever changed.
    ///
    /// # Errors
    /// The file exists but cannot be read, or does not parse.
    pub fn read_settings(&self) -> Result<Option<SetNodeSettings>, SettingsFileError> {
        let path = self.settings_path();
        let text = match std::fs::read(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(StoreError::io(&path)(error).into()),
        };
        serde_json::from_slice(&text)
            .map(Some)
            .map_err(|source| SettingsFileError::Parse { path, source })
    }

    /// Replace the saved settings, owner-only and atomically.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn write_settings(&self, settings: &SetNodeSettings) -> Result<(), StoreError> {
        let path = self.settings_path();
        let text = serde_json::to_vec_pretty(settings)
            .map_err(|error| StoreError::io(&path)(io::Error::other(error)))?;
        files::replace(&path, &text).map_err(StoreError::io(&path))
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::parking::{IdleMinutes, ParkAfter};

    use super::*;

    #[test]
    fn saved_settings_are_read_back_and_a_bad_file_says_why() {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        assert!(data.read_settings().unwrap().is_none());
        let saved = SetNodeSettings {
            park_after: Some(ParkAfter::Idle {
                minutes: IdleMinutes::new(15).unwrap(),
            }),
            drivers: None,
            browser_tools: None,
        };
        data.write_settings(&saved).unwrap();
        assert_eq!(data.read_settings().unwrap(), Some(saved));
        std::fs::write(data.settings_path(), b"{ not json").unwrap();
        assert!(matches!(
            data.read_settings(),
            Err(SettingsFileError::Parse { .. })
        ));
    }
}
