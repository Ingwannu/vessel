//! Real CLI behavior across the named recovery protocol boundary.
use super::support::{Fixture, within};
use vessel_node::DataDir;
use vessel_proto::restart::{RESTART_RECOVERY_PROTOCOL, SetRestartRecovery};
use vessel_proto::settings::SetNodeSettings;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_cli_distinguishes_supported_recovery_from_an_unassigned_protocol() {
    within(async {
        let saved = SetNodeSettings {
            restart_recovery: Some(SetRestartRecovery {
                enabled: Some(true),
                prompt: Some("synthetic continuation".parse().unwrap()),
            }),
            ..SetNodeSettings::default()
        };
        let f = Fixture::start_with(|config, _| {
            config.data_dir.write_settings(&saved).unwrap();
        })
        .await;
        let shown = f.vessel(&["node", "settings"], b"").await;
        assert!(shown.ok, "{}", shown.stderr);
        if RESTART_RECOVERY_PROTOCOL.is_some() {
            assert!(shown.stdout.contains("startup recovery: on"));
            assert!(
                shown
                    .stdout
                    .contains("resume prompt: synthetic continuation")
            );
            let off = f
                .vessel(&["node", "settings", "--resume-on-startup", "off"], b"")
                .await;
            assert!(off.ok, "{}", off.stderr);
            assert!(off.stdout.contains("startup recovery: off"));
            let recovery = DataDir::new(f.data())
                .read_settings()
                .unwrap()
                .unwrap()
                .restart_recovery
                .unwrap();
            assert_eq!(recovery.enabled, Some(false));
            assert_eq!(recovery.prompt, saved.restart_recovery.unwrap().prompt);
            f.stop().await;
            return;
        }
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
            DataDir::new(f.data()).read_settings().unwrap(),
            Some(saved.clone())
        );
        let legacy = f
            .vessel(&["node", "settings", "--park-after", "never"], b"")
            .await;
        assert!(legacy.ok, "{}", legacy.stderr);
        assert_eq!(
            DataDir::new(f.data())
                .read_settings()
                .unwrap()
                .unwrap()
                .restart_recovery,
            saved.restart_recovery
        );
        f.stop().await;
    })
    .await;
}
