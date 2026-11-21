//! A fresh node records its opt-out before opening its first event store.
//! An already-used store keeps the old default without rewriting any file.

use chrono::Utc;
use vessel_proto::mcp::Injection;
use vessel_proto::settings::SetNodeSettings;

use super::{DataDir, SetAside, StoreError};

impl DataDir {
    /// Record the fresh-host tool default under the existing node lock.
    /// Reuse the settings owner's preservation and atomic replacement.
    ///
    /// # Errors
    /// Store detection, preservation or the settings write failed.
    pub fn initialize_vessel_tools(&self) -> Result<(), StoreError> {
        let database = self.database_path();
        if database.try_exists().map_err(StoreError::io(&database))? {
            return Ok(());
        }
        let mut saved = match self.read_settings() {
            Ok(saved) => saved.unwrap_or_default(),
            Err(_) => {
                let _aside = self.set_aside_settings(Utc::now(), SetAside::Unreadable)?;
                SetNodeSettings::default()
            }
        };
        if saved.vessel_tools.is_some() {
            return Ok(());
        }
        if self.settings_hold_more_than(&saved)? {
            let _aside = self.set_aside_settings(Utc::now(), SetAside::Later)?;
        }
        saved.vessel_tools = Some(Injection::Off);
        self.write_settings(&saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_default_is_persisted_while_an_upgrade_keeps_its_existing_file() {
        let fresh = tempfile::tempdir().unwrap();
        let data = DataDir::new(fresh.path());
        data.initialize_vessel_tools().unwrap();
        std::fs::write(data.database_path(), []).unwrap();
        data.initialize_vessel_tools().unwrap();
        assert_eq!(
            data.read_settings().unwrap().unwrap().vessel_tools,
            Some(Injection::Off)
        );

        let upgraded = tempfile::tempdir().unwrap();
        let data = DataDir::new(upgraded.path());
        std::fs::write(data.database_path(), []).unwrap();
        data.initialize_vessel_tools().unwrap();
        assert!(data.read_settings().unwrap().is_none());
    }

    #[test]
    fn a_fresh_node_preserves_a_choice_and_the_settings_copy() {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        let _init_id = data.load_or_create_node_id().unwrap();
        let path = data.settings_path();
        let future = br#"{"park_after":{"kind":"never"},"later":{"keep":true}}"#;
        std::fs::write(&path, future).unwrap();
        data.initialize_vessel_tools().unwrap();
        let saved = data.read_settings().unwrap().unwrap();
        assert_eq!(
            saved.park_after,
            Some(vessel_proto::parking::ParkAfter::Never)
        );
        assert_eq!(saved.vessel_tools, Some(Injection::Off));
        assert!(std::fs::read_dir(root.path()).unwrap().any(|entry| {
            let entry = entry.unwrap();
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("settings.json.later-")
                && std::fs::read(entry.path()).unwrap() == future
        }));
        data.write_settings(&SetNodeSettings {
            vessel_tools: Some(Injection::On),
            ..saved
        })
        .unwrap();
        data.initialize_vessel_tools().unwrap();
        assert_eq!(
            data.read_settings().unwrap().unwrap().vessel_tools,
            Some(Injection::On)
        );
    }
}
