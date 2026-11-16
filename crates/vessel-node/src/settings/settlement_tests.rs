//! Real file/SQL failures must not become saved success or unchanged-value ACKs.

use super::recovery_apply::RecoveryApplyError;
use super::writer_tests::{RecoveryChoice, WRITE_TEST_BUDGET, patch, recovery, settings};
use super::*;
use vessel_proto::restart::SetRestartRecovery;
use vessel_store::StoreError;

#[tokio::test]
async fn failed_actual_off_replacement_keeps_the_previous_saved_policy_and_can_retry() {
    tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = settings(root.path()).await;
        let _saved = settings.change(recovery(RecoveryChoice::On)).await.unwrap();
        let path = settings.state.data_dir.settings_path();
        let original = std::fs::read(&path).unwrap();
        let kept = root.path().join("previous-settings");
        std::fs::rename(&path, &kept).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(matches!(
            settings.change(recovery(RecoveryChoice::Off)).await,
            Err(SettingsError::Write(_))
        ));
        assert!(settings.current().await.restart_recovery.enabled);
        assert_eq!(std::fs::read(&kept).unwrap(), original);
        std::fs::remove_dir(&path).unwrap();
        std::fs::rename(&kept, &path).unwrap();
        assert!(
            !settings
                .change(recovery(RecoveryChoice::Off))
                .await
                .unwrap()
                .restart_recovery
                .enabled
        );
        let saved = settings
            .state
            .data_dir
            .read_settings()
            .unwrap()
            .unwrap()
            .public_settings();
        assert!(!saved.restart_recovery.unwrap().effective().enabled);
        settings.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn failed_cleanup_keeps_saved_off_and_blocks_later_on_before_any_replacement() {
    tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = settings(root.path()).await;
        let _saved = settings.change(recovery(RecoveryChoice::On)).await.unwrap();
        settings.state.recovery.store().close().await;
        assert!(matches!(
            settings.change(recovery(RecoveryChoice::Off)).await,
            Err(SettingsError::Recovery(RecoveryApplyError::Cleanup(
                StoreError::Closed
            )))
        ));
        assert!(!settings.current().await.restart_recovery.enabled);
        let path = settings.state.data_dir.settings_path();
        let off = std::fs::read(&path).unwrap();
        let (_entered, _release) = settings.pause_write();
        for change in [
            recovery(RecoveryChoice::On),
            recovery(RecoveryChoice::Off),
            patch(ParkAfter::Never),
        ] {
            assert!(matches!(
                settings.change(change).await,
                Err(SettingsError::Recovery(RecoveryApplyError::Cleanup(
                    StoreError::Closed
                )))
            ));
            assert_eq!(std::fs::read(&path).unwrap(), off);
        }
        assert!(
            settings.state.gate.take().is_some(),
            "no later operation reached the real writer"
        );
        assert!(!settings.current().await.restart_recovery.enabled);
        settings.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn failed_on_apply_retries_an_identical_value_without_rewriting_or_false_success() {
    tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = settings(root.path()).await;
        settings.state.recovery.store().close().await;
        assert!(matches!(
            settings.change(recovery(RecoveryChoice::On)).await,
            Err(SettingsError::Recovery(RecoveryApplyError::Apply(
                StoreError::Closed
            )))
        ));
        let path = settings.state.data_dir.settings_path();
        let on = std::fs::read(&path).unwrap();
        let (_entered, _release) = settings.pause_write();
        assert!(matches!(
            settings.change(recovery(RecoveryChoice::On)).await,
            Err(SettingsError::Recovery(RecoveryApplyError::Apply(
                StoreError::Closed
            )))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), on);
        assert!(
            settings.state.gate.take().is_some(),
            "the same saved value retried SQL without another replacement"
        );
        assert!(settings.current().await.restart_recovery.enabled);
        settings.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn ordinary_patches_preserve_the_saved_recovery_patch_and_verbatim_prompt() {
    tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = settings(root.path()).await;
        let recovery = SetRestartRecovery {
            enabled: Some(true),
            prompt: Some("  Preserve exactly.\n계속.\t".parse().unwrap()),
        };
        let _saved = settings
            .change(SetNodeSettings {
                restart_recovery: Some(recovery.clone()),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        let _saved = settings.change(patch(ParkAfter::Never)).await.unwrap();
        let _saved = settings
            .change(SetNodeSettings {
                drivers: Some(SetAgentDrivers::default()),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        let saved = settings
            .state
            .data_dir
            .read_settings()
            .unwrap()
            .unwrap()
            .public_settings();
        assert_eq!(saved.restart_recovery, Some(recovery.clone()));
        assert_eq!(
            settings.current().await.restart_recovery,
            recovery.effective()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(settings.state.data_dir.settings_path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        settings.shutdown().await;
    })
    .await
    .unwrap();
}
