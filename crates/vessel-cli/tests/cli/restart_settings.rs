//! Real CLI behavior while recovery's feature number remains unassigned.
use super::support::{Fixture, within};
use vessel_node::DataDir;
use vessel_proto::restart::SetRestartRecovery;
use vessel_store::SavedNodeSettings;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_recovery_is_not_reported_as_off_and_cannot_change_saved_policy() {
    within(async {
        let saved = SavedNodeSettings {
            restart_recovery: Some(SetRestartRecovery {
                enabled: Some(true),
                prompt: Some("synthetic continuation".parse().unwrap()),
            }),
            ..SavedNodeSettings::default()
        };
        let f = Fixture::start_with(|config, _| {
            config.data_dir.write_settings_record(&saved).unwrap();
        })
        .await;
        let shown = f.vessel(&["node", "settings"], b"").await;
        assert!(shown.ok, "{}", shown.stderr);
        assert!(
            shown.stdout.contains("startup recovery: unsupported"),
            "{}",
            shown.stdout
        );
        assert!(!shown.stdout.contains("startup recovery: off"));
        let refused = f
            .vessel(&["node", "settings", "--resume-on-startup", "off"], b"")
            .await;
        assert!(!refused.ok);
        assert!(
            refused.stderr.contains("not assigned"),
            "{}",
            refused.stderr
        );
        assert_eq!(
            DataDir::new(f.data()).read_settings_record().unwrap(),
            Some(saved.clone())
        );
        let legacy = f
            .vessel(&["node", "settings", "--park-after", "never"], b"")
            .await;
        assert!(legacy.ok, "{}", legacy.stderr);
        assert_eq!(
            DataDir::new(f.data())
                .read_settings_record()
                .unwrap()
                .unwrap()
                .restart_recovery,
            saved.restart_recovery
        );
        f.stop().await;
    })
    .await;
}
