//! The plugin host's files in the data directory (ADR 0115): the list of
//! installations and the plugins' settings documents, which the node
//! writes for its helpers (ADR 0115, "Settings": a helper never writes its
//! own settings files).
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

use vessel_proto::plugins::{PluginId, SettingsId};

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
/// A settings document larger than this is not read: its values fit the
/// helper payload budget (`MAX_HELPER_PAYLOAD_BYTES`, 1 MiB), and the
/// envelope adds a few bytes.
const MAX_SETTINGS_FILE_BYTES: u64 = 1024 * 1024 + 4096;

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

    /// One settings document's file (`plugin-settings/<id>/<settings>.json`).
    #[must_use]
    pub fn plugin_settings_path(&self, plugin: &PluginId, settings: &SettingsId) -> PathBuf {
        self.plugin_settings_dir(plugin)
            .join(format!("{}.json", settings.as_str()))
    }

    /// A settings document's bytes; `None` when it was never saved.
    /// Blocking.
    ///
    /// # Errors
    /// It cannot be read, or is larger than its bound.
    pub fn read_plugin_settings(
        &self,
        plugin: &PluginId,
        settings: &SettingsId,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        bounded_read(
            &self.plugin_settings_path(plugin, settings),
            MAX_SETTINGS_FILE_BYTES,
        )
    }

    /// Replace a settings document with `contents`, owner-only (0700
    /// folders, 0600 file on Unix), atomically. Blocking.
    ///
    /// # Errors
    /// The folder or the file cannot be written.
    pub fn write_plugin_settings(
        &self,
        plugin: &PluginId,
        settings: &SettingsId,
        contents: &[u8],
    ) -> Result<(), StoreError> {
        let dir = self.plugin_settings_dir(plugin);
        files::create_private_dir(&dir).map_err(StoreError::io(&dir))?;
        let path = self.plugin_settings_path(plugin, settings);
        files::replace(&path, contents).map_err(StoreError::io(&path))
    }

    /// The installations file's bytes; `None` when there is none yet.
    /// Blocking.
    ///
    /// # Errors
    /// It cannot be read, or is larger than [`MAX_INSTALLATIONS_BYTES`].
    pub fn read_plugin_installations(&self) -> Result<Option<Vec<u8>>, StoreError> {
        bounded_read(&self.plugin_installations_path(), MAX_INSTALLATIONS_BYTES)
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

/// `path`'s bytes when it exists and is at most `limit` long. Blocking.
fn bounded_read(path: &std::path::Path, limit: u64) -> Result<Option<Vec<u8>>, StoreError> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(StoreError::io(path)(error)),
    };
    if metadata.len() > limit {
        return Err(StoreError::io(path)(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("larger than {limit} bytes"),
        )));
    }
    std::fs::read(path).map(Some).map_err(StoreError::io(path))
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
    fn a_settings_document_round_trips_owner_only_and_goes_with_its_folder() {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        let plugin: PluginId = "notes".parse().unwrap();
        let settings: SettingsId = "display".parse().unwrap();
        assert_eq!(data.read_plugin_settings(&plugin, &settings).unwrap(), None);
        data.write_plugin_settings(&plugin, &settings, b"{\"version\":1,\"values\":{}}")
            .unwrap();
        assert_eq!(
            data.read_plugin_settings(&plugin, &settings).unwrap(),
            Some(b"{\"version\":1,\"values\":{}}".to_vec())
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let file = std::fs::metadata(data.plugin_settings_path(&plugin, &settings)).unwrap();
            assert_eq!(file.permissions().mode() & 0o777, 0o600);
            let dir = std::fs::metadata(data.plugin_settings_dir(&plugin)).unwrap();
            assert_eq!(dir.permissions().mode() & 0o777, 0o700);
        }
        data.remove_plugin_settings(&plugin).unwrap();
        assert_eq!(data.read_plugin_settings(&plugin, &settings).unwrap(), None);
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
