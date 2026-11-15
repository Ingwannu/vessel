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
        use vessel_proto::mcp::Injection;
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
        if saved.mcp.as_ref().is_some_and(|mcp| mcp.inject.is_some()) {
            return Ok(());
        }
        if self.settings_hold_more_than(&saved)? {
            let _aside = self.set_aside_settings(chrono::Utc::now(), SetAside::Later)?;
        }
        // A provider policy is not an injection choice. Fill only the missing
        // switch so an upgrade does not silently disable an existing policy.
        saved.mcp.get_or_insert_default().inject = Some(if existing {
            Injection::On
        } else {
            Injection::Off
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
    fn a_policy_without_a_switch_keeps_its_policy_and_gets_the_identity_default() {
        for existing in [false, true] {
            let home = tempfile::tempdir().unwrap();
            let data = DataDir::new(home.path());
            if existing {
                let _identity = data.load_or_create_node_id().unwrap();
            }
            let original =
                br#"{"mcp":{"policy":[{"harness":"claude","enabled":false,"disabled":[]}]}}"#;
            std::fs::write(data.settings_path(), original).unwrap();
            let before = data.read_settings().unwrap().unwrap().mcp.unwrap().policy;
            data.initialize_tool_settings().unwrap();
            let after = data.read_settings().unwrap().unwrap().mcp.unwrap();
            assert_eq!(after.policy, before);
            assert_eq!(
                after.inject,
                Some(if existing {
                    Injection::On
                } else {
                    Injection::Off
                })
            );
            assert_eq!(data.read_node_id().unwrap().is_some(), existing);
        }
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
