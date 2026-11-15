//! Fresh/upgrade injection migration runs under the node lock before identity creation.
//! Settings codecs and protective backups remain the existing store implementation.

use crate::data_dir::NODE_ID_FILE;
use crate::{DataDir, StoreError};

impl DataDir {
    /// Freeze fresh/upgrade tool injection before this start creates node.id.
    /// The node holds its data lock; a failed preservation/write refuses start.
    ///
    /// # Errors
    /// Identity presence, preservation of earlier settings or persistence failed.
    pub fn initialize_tool_settings(&self) -> Result<(), StoreError> {
        use crate::SetAside;
        use vessel_proto::mcp::{Injection, McpSettingsPatch};
        let existing = match std::fs::symlink_metadata(self.root().join(NODE_ID_FILE)) {
            Ok(_entry) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(StoreError::io(self.root().join(NODE_ID_FILE))(error)),
        };
        let path = self.settings_path();
        match std::fs::symlink_metadata(&path) {
            Ok(info) if info.is_file() && !info.is_symlink() => {}
            Ok(_other) => {
                return Err(StoreError::io(&path)(std::io::Error::other(
                    "settings must be a plain file",
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StoreError::io(&path)(error)),
        }
        let mut saved = match self.read_settings() {
            Ok(saved) => saved.unwrap_or_default(),
            Err(_unusable) => {
                let _aside = self.set_aside_settings(chrono::Utc::now(), SetAside::Unreadable)?;
                vessel_proto::settings::SetNodeSettings::default()
            }
        };
        if saved.mcp.is_some() {
            return Ok(());
        }
        if self.settings_hold_more_than(&saved)? {
            let _aside = self.set_aside_settings(chrono::Utc::now(), SetAside::Later)?;
        }
        saved.mcp = Some(McpSettingsPatch {
            inject: Some(if existing {
                Injection::On
            } else {
                Injection::Off
            }),
            policy: None,
        });
        self.write_settings(&saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::mcp::Injection;

    fn injection(data: &DataDir) -> Injection {
        data.read_settings()
            .unwrap()
            .unwrap()
            .mcp
            .unwrap()
            .settings()
            .inject
    }

    #[test]
    fn fresh_starts_off_before_identity_and_an_upgrade_keeps_tools_on() {
        let fresh = tempfile::tempdir().unwrap();
        let data = DataDir::new(fresh.path());
        data.initialize_tool_settings().unwrap();
        assert_eq!(injection(&data), Injection::Off);
        assert!(data.read_node_id().unwrap().is_none());
        let _identity = data.load_or_create_node_id().unwrap();
        data.initialize_tool_settings().unwrap();
        assert_eq!(
            injection(&data),
            Injection::Off,
            "explicit false is never an upgrade signal"
        );
        let old = tempfile::tempdir().unwrap();
        let data = DataDir::new(old.path());
        let _identity = data.load_or_create_node_id().unwrap();
        data.initialize_tool_settings().unwrap();
        assert_eq!(injection(&data), Injection::On);
    }

    #[test]
    fn upgrade_preserves_unknown_or_unreadable_saved_settings() {
        let home = tempfile::tempdir().unwrap();
        let data = DataDir::new(home.path());
        let _identity = data.load_or_create_node_id().unwrap();
        let original = br#"{"later":{"keep":"value"}}"#;
        std::fs::write(data.settings_path(), original).unwrap();
        data.initialize_tool_settings().unwrap();
        assert_eq!(injection(&data), Injection::On);
        let copies: Vec<_> = std::fs::read_dir(home.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.json.later-")
            })
            .collect();
        assert_eq!(copies.len(), 1);
        assert_eq!(std::fs::read(copies[0].path()).unwrap(), original);
        std::fs::write(data.settings_path(), b"malformed").unwrap();
        data.initialize_tool_settings().unwrap();
        assert_eq!(injection(&data), Injection::On);
        assert!(
            std::fs::read_dir(home.path())
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.json.unreadable-"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn linked_settings_are_refused_before_identity_or_permission_changes() {
        use std::os::unix::fs::PermissionsExt as _;
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"preserve").unwrap();
        let mode = std::fs::metadata(outside.path())
            .unwrap()
            .permissions()
            .mode();
        let data = DataDir::new(home.path());
        std::os::unix::fs::symlink(outside.path(), data.settings_path()).unwrap();
        assert!(data.initialize_tool_settings().is_err());
        assert!(data.read_node_id().unwrap().is_none());
        assert_eq!(std::fs::read(outside.path()).unwrap(), b"preserve");
        assert_eq!(
            std::fs::metadata(outside.path())
                .unwrap()
                .permissions()
                .mode(),
            mode
        );
    }
}
