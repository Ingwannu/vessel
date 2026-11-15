//! Config uses live typed settings and validates before attempting a connection.

use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::GetNodeSettings;

use crate::support::{Fixture, within};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn whole_field_get_and_set_use_the_live_node_and_keep_the_change() {
    within(async {
        let fixture = Fixture::start().await;
        let ran = fixture
            .vessel(
                &["node", "config", "set", "park_after", r#"{"kind":"never"}"#],
                b"",
            )
            .await;
        assert!(ran.ok, "{}", ran.stderr);
        assert_eq!(
            serde_json::from_str::<ParkAfter>(&ran.stdout).unwrap(),
            ParkAfter::Never
        );
        let ran = fixture
            .vessel(&["node", "config", "get", "park_after"], b"")
            .await;
        assert!(ran.ok, "{}", ran.stderr);
        assert_eq!(
            serde_json::from_str::<ParkAfter>(&ran.stdout).unwrap(),
            ParkAfter::Never
        );
        let fixture = fixture.restart().await;
        assert_eq!(
            fixture
                .owner()
                .await
                .call(GetNodeSettings {})
                .await
                .unwrap()
                .park_after,
            ParkAfter::Never
        );
        let ran = fixture.vessel(&["node", "config", "get"], b"").await;
        assert!(ran.ok, "{}", ran.stderr);
        let settings: vessel_proto::settings::NodeSettings =
            serde_json::from_str(&ran.stdout).unwrap();
        assert_eq!(settings.park_after, ParkAfter::Never);
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_values_and_dotted_or_startup_keys_cannot_mutate_settings() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let before = owner.call(GetNodeSettings {}).await.unwrap();
        for (key, value) in [
            ("park_after", r#"{"kind":"private-config-canary"}"#),
            ("drivers", r#"{"codex":"later"}"#),
            ("drivers.codex", r#""acp""#),
            ("bind", r#""127.0.0.1:0""#),
        ] {
            let ran = fixture
                .vessel(&["node", "config", "set", key, value], b"")
                .await;
            assert!(!ran.ok);
            assert!(!ran.stderr.contains("private-config-canary"));
            assert_eq!(owner.call(GetNodeSettings {}).await.unwrap(), before);
        }
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn value_validation_precedes_even_local_credential_resolution() {
    within(async {
        let empty = tempfile::tempdir().unwrap();
        let ran = crate::support::vessel_in(
            empty.path(),
            &[
                "node",
                "config",
                "set",
                "park_after",
                r#"{"kind":"private-config-canary"}"#,
            ],
            b"",
        )
        .await;
        assert!(!ran.ok);
        assert!(ran.stderr.contains("setting value"), "{}", ran.stderr);
        assert!(!ran.stderr.contains("private-config-canary"));
        assert!(!ran.stderr.contains("no token"));
        assert!(!empty.path().join("settings.json").exists());
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_mutations_follow_the_assignment_and_keep_their_state_contract() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let saved = owner
            .call(vessel_proto::settings::SetNodeSettings {
                park_after: Some(ParkAfter::Never),
                ..Default::default()
            })
            .await
            .unwrap();
        let unset = fixture
            .vessel(&["node", "config", "unset", "park_after"], b"")
            .await;
        let reload = fixture.vessel(&["node", "reload"], b"").await;
        match vessel_proto::settings::NODE_CONFIG_PROTOCOL {
            None => {
                for ran in [unset, reload] {
                    assert!(!ran.ok);
                    assert!(
                        ran.stderr.contains("protocol version is not assigned"),
                        "{}",
                        ran.stderr
                    );
                }
                assert_eq!(owner.call(GetNodeSettings {}).await.unwrap(), saved);
            }
            Some(_) => {
                assert!(unset.ok, "{}", unset.stderr);
                assert!(reload.ok, "{}", reload.stderr);
                let current: vessel_proto::settings::NodeSettings =
                    serde_json::from_str(&unset.stdout).unwrap();
                assert_eq!(current.park_after, ParkAfter::DEFAULT);
                assert_eq!(owner.call(GetNodeSettings {}).await.unwrap(), current);
                let report: vessel_proto::settings::ReloadedNodeSettings =
                    serde_json::from_str(&reload.stdout).unwrap();
                assert_eq!(report, Default::default());
            }
        }
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_members_refuse_before_credentials_or_contact() {
    within(async {
        let empty = tempfile::tempdir().unwrap();
        for (key, input) in [
            (
                "drivers",
                r#"{"codex":"private-duplicate-cli-canary","codex":"acp"}"#,
            ),
            ("drivers", r#"{"codex":null,"codex":"acp"}"#),
            (
                "park_after",
                r#"{"kind":"private-duplicate-cli-canary","kind":"never"}"#,
            ),
        ] {
            let ran = crate::support::vessel_in(
                empty.path(),
                &["node", "config", "set", key, input],
                b"",
            )
            .await;
            assert!(!ran.ok);
            assert!(ran.stderr.contains("setting value"), "{}", ran.stderr);
            assert!(!ran.stderr.contains("private-duplicate-cli-canary"));
            assert!(!ran.stderr.contains("no token"));
            assert!(!empty.path().join("settings.json").exists());
        }
    })
    .await;
}
