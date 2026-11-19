//! The plugin host's files in the data directory (ADR 0115): the list of
//! installations and the plugins' own settings.
//!
//! [Decision Log]
//! - Purpose: Paseo keeps installations in its `config.json` under
//!   `plugins`; Vessel's node keeps them in `plugins/installations.json`,
//!   written by the plugin host alone, so the node's `settings.json` keeps
//!   one writer (ADR 0115).
//! - Choice: the store owns only where the files are and how they are
//!   written (owner-only directory and file, replaced atomically); the
//!   node owns their shape. Bytes in, bytes out.
//! - A file that cannot be read is moved aside with the time in its name,
//!   as `settings.json` is, so a person's list is never overwritten.

use std::path::PathBuf;

use vessel_proto::plugins::PluginId;

use crate::DataDir;
use crate::error::StoreError;
use crate::files;

/// Folder of the plugin host's own files.
const PLUGINS_DIR: &str = "plugins";
/// The installations, inside [`PLUGINS_DIR`].
const INSTALLATIONS_FILE: &str = "installations.json";
/// Folder of the plugins' settings, one folder per installation id.
const PLUGIN_SETTINGS_DIR: &str = "plugin-settings";
/// An installations file larger than this is not read (a person's list of
/// plugins is a few hundred bytes a row).
const MAX_INSTALLATIONS_BYTES: u64 = 1024 * 1024;

impl DataDir {
    /// The plugin host's folder (`plugins/`).
    #[must_use]
    pub fn plugins_dir(&self) -> PathBuf {
        self.root().join(PLUGINS_DIR)
    }

    /// Where the installations are kept.
    #[must_use]
    pub fn plugin_installations_path(&self) -> PathBuf {
        self.plugins_dir().join(INSTALLATIONS_FILE)
    }

    /// One installation's settings folder (`plugin-settings/<id>/`).
    #[must_use]
    pub fn plugin_settings_dir(&self, plugin: &PluginId) -> PathBuf {
        self.root().join(PLUGIN_SETTINGS_DIR).join(plugin.as_str())
    }

    /// The installations file's bytes; `None` when there is none yet.
    /// Blocking.
    ///
    /// # Errors
    /// It cannot be read, or is larger than [`MAX_INSTALLATIONS_BYTES`].
    pub fn read_plugin_installations(&self) -> Result<Option<Vec<u8>>, StoreError> {
        let path = self.plugin_installations_path();
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(StoreError::io(&path)(error)),
        };
        if metadata.len() > MAX_INSTALLATIONS_BYTES {
            return Err(StoreError::io(&path)(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("larger than {MAX_INSTALLATIONS_BYTES} bytes"),
            )));
        }
        std::fs::read(&path)
            .map(Some)
            .map_err(StoreError::io(&path))
    }

    /// Replace the installations file with `contents`, owner-only (0700
    /// folder, 0600 file on Unix), atomically. Blocking.
    ///
    /// # Errors
    /// The folder or the file cannot be written.
    pub fn write_plugin_installations(&self, contents: &[u8]) -> Result<(), StoreError> {
        let dir = self.plugins_dir();
        files::create_private_dir(&dir).map_err(StoreError::io(&dir))?;
        let path = self.plugin_installations_path();
        files::replace(&path, contents).map_err(StoreError::io(&path))
    }

    /// Move an unreadable installations file aside
    /// (`installations.json.unreadable-<UTC time>`). Blocking.
    ///
    /// # Errors
    /// It cannot be renamed.
    pub fn set_aside_plugin_installations(
        &self,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<PathBuf, StoreError> {
        let path = self.plugin_installations_path();
        let aside = path.with_file_name(format!(
            "{INSTALLATIONS_FILE}.unreadable-{}",
            at.format("%Y%m%dT%H%M%SZ")
        ));
        std::fs::rename(&path, &aside).map_err(StoreError::io(&aside))?;
        Ok(aside)
    }

    /// Delete one installation's settings folder; nothing when it has
    /// none. Blocking.
    ///
    /// # Errors
    /// It exists and cannot be removed.
    pub fn remove_plugin_settings(&self, plugin: &PluginId) -> Result<(), StoreError> {
        let dir = self.plugin_settings_dir(plugin);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(StoreError::io(&dir)(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installations_round_trip_owner_only_and_move_aside() {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        assert_eq!(data.read_plugin_installations().unwrap(), None);
        data.write_plugin_installations(b"{\"a\":1}").unwrap();
        assert_eq!(
            data.read_plugin_installations().unwrap().as_deref(),
            Some(&b"{\"a\":1}"[..])
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let file = std::fs::metadata(data.plugin_installations_path()).unwrap();
            assert_eq!(file.permissions().mode() & 0o777, 0o600);
            let dir = std::fs::metadata(data.plugins_dir()).unwrap();
            assert_eq!(dir.permissions().mode() & 0o777, 0o700);
        }
        let aside = data
            .set_aside_plugin_installations(chrono::Utc::now())
            .unwrap();
        assert!(aside.exists());
        assert_eq!(data.read_plugin_installations().unwrap(), None);
    }

    #[test]
    fn removing_settings_is_quiet_when_there_are_none() {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        let plugin: PluginId = "theme".parse().unwrap();
        data.remove_plugin_settings(&plugin).unwrap();
        let dir = data.plugin_settings_dir(&plugin);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.json"), b"{}").unwrap();
        data.remove_plugin_settings(&plugin).unwrap();
        assert!(!dir.exists());
    }
}
