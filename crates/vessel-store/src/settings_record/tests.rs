//! Saved-record compatibility, privacy and the existing file coverage boundary.
use super::*;
use std::error::Error as _;

fn chosen() -> SavedNodeSettings {
    SavedNodeSettings {
        park_after: Some(ParkAfter::Never),
        drivers: None,
        restart_recovery: Some(SetRestartRecovery {
            enabled: Some(true),
            prompt: Some("  계속하세요.\r\n\t".parse().unwrap()),
        }),
    }
}

#[test]
fn full_record_keeps_verbatim_policy_and_the_old_view_still_decodes() {
    let directory = tempfile::tempdir().unwrap();
    let data = DataDir::new(directory.path());
    assert_eq!(data.read_settings_record().unwrap(), None);
    let record = chosen();
    data.write_settings_record(&record).unwrap();
    assert_eq!(data.read_settings_record().unwrap(), Some(record.clone()));
    assert_eq!(
        data.read_settings().unwrap(),
        Some(SetNodeSettings {
            park_after: Some(ParkAfter::Never),
            drivers: None,
            ..SetNodeSettings::default()
        })
    );
    assert!(!data.settings_record_holds_more_than(&record).unwrap());
    assert!(
        data.settings_hold_more_than(&data.read_settings().unwrap().unwrap())
            .unwrap()
    );
}

#[test]
fn full_record_still_detects_unknown_nested_policy_before_replacing_it() {
    let directory = tempfile::tempdir().unwrap();
    let data = DataDir::new(directory.path());
    std::fs::write(
        data.settings_path(),
        br#"{"restart_recovery":{"enabled":true,"later_field":"keep separately"}}"#,
    )
    .unwrap();
    let record = data.read_settings_record().unwrap().unwrap();
    assert!(data.settings_record_holds_more_than(&record).unwrap());
    let before = std::fs::read(data.settings_path()).unwrap();
    let aside = data
        .set_aside_settings(chrono::Utc::now(), crate::SetAside::Later)
        .unwrap();
    assert_eq!(std::fs::read(aside).unwrap(), before);
    assert_eq!(std::fs::read(data.settings_path()).unwrap(), before);
}

#[test]
fn malformed_record_diagnostics_redact_input_and_keep_the_original_source() {
    const PRIVATE: &str = "synthetic-private-recovery-note";
    let directory = tempfile::tempdir().unwrap();
    let data = DataDir::new(directory.path());
    let text = format!(r#"{{"restart_recovery":{{"enabled":"{PRIVATE}"}}}}"#);
    std::fs::write(data.settings_path(), text).unwrap();
    let error = data.read_settings_record().unwrap_err();
    assert!(matches!(error, SettingsFileError::RecordParse { .. }));
    assert!(!error.to_string().contains(PRIVATE));
    assert!(!format!("{error:?}").contains(PRIVATE));
    assert!(error.source().is_some());
}

#[test]
fn a_legacy_patch_conversion_contains_no_recovery_change() {
    let patch = SetNodeSettings {
        park_after: Some(ParkAfter::Never),
        drivers: None,
        ..SetNodeSettings::default()
    };
    assert_eq!(
        SavedNodeSettings::from(patch),
        SavedNodeSettings {
            park_after: patch.park_after,
            drivers: patch.drivers,
            restart_recovery: None,
        }
    );
}

#[cfg(unix)]
#[test]
fn full_record_replacement_keeps_the_file_private() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = tempfile::tempdir().unwrap();
    let data = DataDir::new(directory.path());
    std::fs::write(data.settings_path(), b"{}").unwrap();
    std::fs::set_permissions(data.settings_path(), std::fs::Permissions::from_mode(0o644)).unwrap();
    data.write_settings_record(&chosen()).unwrap();
    assert_eq!(
        std::fs::metadata(data.settings_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
}
