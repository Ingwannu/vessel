//! Real settings persistence and runtime watches, without launching an agent.

use super::*;

fn settings(root: &std::path::Path, flag: ParkAfter) -> Settings {
    let runtime = SessionRuntime::new(
        vessel_session::RuntimeConfig {
            harness_lookup: vessel_harness::Lookup::new(std::iter::empty::<std::path::PathBuf>()),
            harness_env: vessel_harness::HarnessEnv::new(root.to_owned()),
            ring_capacity: vessel_session::DEFAULT_RING_CAPACITY,
            durable_capacity: vessel_session::DEFAULT_DURABLE_CAPACITY,
            agent_handshake_timeout: vessel_session::DEFAULT_HANDSHAKE_TIMEOUT,
            park_after: flag.duration(),
            private_dir: None,
        },
        tokio_util::sync::CancellationToken::new(),
    );
    Settings::open(DataDir::new(root), Arc::new(runtime), flag, false)
}

#[tokio::test]
async fn reset_persists_override_removal_and_reapplies_the_launch_value() {
    let root = tempfile::tempdir().unwrap();
    let launch = ParkAfter::Never;
    let saved = ParkAfter::DEFAULT;
    let settings = settings(root.path(), launch);
    settings
        .change(SetNodeSettings {
            park_after: Some(saved),
            ..SetNodeSettings::default()
        })
        .await
        .unwrap();
    assert_eq!(settings.runtime.park_after(), saved.duration());
    let keys = SettingKeys::new(vec![NodeSettingKey::ParkAfter]).unwrap();
    let current = settings.unset(&keys).await.unwrap();
    assert_eq!(current.park_after, launch);
    assert_eq!(settings.runtime.park_after(), launch.duration());
    assert_eq!(
        settings
            .data_dir
            .read_settings()
            .unwrap()
            .unwrap()
            .park_after,
        None
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(settings.data_dir.settings_path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);
    }
}

#[tokio::test]
async fn reload_reports_effective_changes_and_missing_file_restores_flags() {
    let root = tempfile::tempdir().unwrap();
    let settings = settings(root.path(), ParkAfter::DEFAULT);
    std::fs::write(
        settings.data_dir.settings_path(),
        br#"{"park_after":{"kind":"never"}}"#,
    )
    .unwrap();
    let report = settings.reload().await.unwrap();
    assert_eq!(report.applied, [NodeSettingKey::ParkAfter]);
    assert!(report.restart_required.is_empty());
    assert!(report.override_controlled.is_empty());
    assert_eq!(settings.runtime.park_after(), None);
    assert!(settings.reload().await.unwrap().applied.is_empty());
    std::fs::remove_file(settings.data_dir.settings_path()).unwrap();
    assert_eq!(
        settings.reload().await.unwrap().applied,
        [NodeSettingKey::ParkAfter]
    );
    assert_eq!(settings.runtime.park_after(), ParkAfter::DEFAULT.duration());
    assert!(!settings.data_dir.settings_path().exists());
}

#[tokio::test]
async fn invalid_reload_and_failed_reset_publish_nothing() {
    let root = tempfile::tempdir().unwrap();
    let settings = settings(root.path(), ParkAfter::DEFAULT);
    let before = settings.current().await;
    let path = settings.data_dir.settings_path();
    for bytes in [
        b"{".as_slice(),
        br#"{"future":"private-config-canary"}"#,
        br#"{"park_after":{"kind":"never"},"drivers":{"codex":"private-config-canary"}}"#,
    ] {
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(
            settings.reload().await,
            Err(SettingsError::Reload(_))
        ));
        assert_eq!(settings.current().await, before);
        assert_eq!(settings.runtime.park_after(), before.park_after.duration());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    std::fs::remove_file(&path).unwrap();
    settings
        .change(SetNodeSettings {
            park_after: Some(ParkAfter::Never),
            ..SetNodeSettings::default()
        })
        .await
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let keys = SettingKeys::new(vec![NodeSettingKey::ParkAfter]).unwrap();
    assert!(matches!(
        settings.unset(&keys).await,
        Err(SettingsError::Write(_))
    ));
    assert_eq!(settings.current().await.park_after, ParkAfter::Never);
    assert_eq!(settings.runtime.park_after(), None);
}
