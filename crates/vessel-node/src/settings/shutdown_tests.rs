//! Physical settlement precedes runtime/Store close; drain seals effects at once.

use crate::settings::writer_tests::phase;
use std::sync::Arc;
use std::task::Poll;

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::StopWhen;

use super::writer_tests::{RecoveryChoice, WRITE_TEST_BUDGET, recovery, runtime_with_stop};
use super::*;

struct StopOnDrop(crate::lifecycle::Lifecycle);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        // A failed observer still asks the real owner to drain. This is a stop
        // request, never a synthetic completion or cancellation of physical I/O.
        self.0.stop(StopWhen::Now);
    }
}

#[tokio::test]
async fn teardown_drains_saved_off_and_queued_on_before_runtime_and_store_close() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let runtime_stop = CancellationToken::new();
        let settings = Arc::new(
            phase(
                "open real settings and Store",
                Settings::open(
                    DataDir::new(root.path()),
                    runtime_with_stop(root.path(), runtime_stop.clone()),
                    ParkAfter::DEFAULT,
                    false,
                ),
            )
            .await,
        );
        let _saved = phase(
            "save On policy",
            settings.change(recovery(RecoveryChoice::On)),
        )
        .await
        .unwrap();
        let store = settings.state.recovery.store();
        let (entered, release) = settings.pause_cleanup();
        let first = Arc::clone(&settings);
        let caller = tokio::spawn(async move { first.change(recovery(RecoveryChoice::Off)).await });
        phase("enter real blocking writer", entered).await.unwrap();
        caller.abort();
        assert!(
            phase("cancelled caller joins", caller)
                .await
                .unwrap_err()
                .is_cancelled()
        );
        let saved_off = settings
            .state
            .data_dir
            .read_settings()
            .unwrap()
            .unwrap()
            .public_settings();
        assert!(!saved_off.restart_recovery.unwrap().effective().enabled);
        assert!(settings.state.saved.try_lock().is_err());
        assert!(!settings.state.recovery.current().enabled);
        let mut queued_on = Box::pin(settings.change(recovery(RecoveryChoice::On)));
        assert!(matches!(futures_util::poll!(&mut queued_on), Poll::Pending));
        let teardown = crate::shutdown::Teardown {
            runtime: Arc::clone(&settings.state.runtime),
            settings: Arc::clone(&settings),
            tasks: TaskTracker::new(),
            forwarders: TaskTracker::new(),
            closing: CancellationToken::new(),
            connections: TaskTracker::new(),
            store: store.clone(),
        };
        let mut stop = Box::pin(teardown.run());
        assert!(matches!(futures_util::poll!(&mut stop), Poll::Pending));
        assert!(!runtime_stop.is_cancelled());
        assert!(phase("Store remains live", store.sessions()).await.is_ok());
        assert!(matches!(
            settings.admission.submit(recovery(RecoveryChoice::On)),
            Err(SettingsError::Stopping)
        ));
        release.send(()).unwrap();
        assert!(
            phase("queued On settlement", queued_on)
                .await
                .unwrap()
                .restart_recovery
                .enabled
        );
        phase("teardown physical drain", stop).await;
        assert!(runtime_stop.is_cancelled());
        // close() settles the writer; retained read-pool clones may still read
        // committed snapshots. A real writer request is the closure receipt.
        assert!(matches!(
            phase(
                "closed Store refuses policy writes",
                store.set_restart_policy(false)
            )
            .await,
            Err(vessel_store::StoreError::Closed)
        ));
        assert!(matches!(
            settings.admission.submit(recovery(RecoveryChoice::On)),
            Err(SettingsError::Stopping)
        ));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn now_and_drain_close_settings_before_the_idle_waiter_polls() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        for when in [StopWhen::Now, StopWhen::WhenIdle] {
            let root = tempfile::tempdir().unwrap();
            let settings = phase(
                "open real settings owner",
                writer_tests::settings(root.path()),
            )
            .await;
            let _saved = phase(
                "save On policy",
                settings.change(recovery(RecoveryChoice::On)),
            )
            .await
            .unwrap();
            let stop = CancellationToken::new();
            let info = vessel_proto::NodeInfo {
                id: vessel_proto::NodeId::random(),
                name: "fixture".to_owned(),
                hostname: "fixture".to_owned(),
                os: "fixture".to_owned(),
                arch: "fixture".to_owned(),
                version: "fixture".to_owned(),
                protocol: vessel_proto::ProtocolRange::CURRENT,
                started_at: chrono::DateTime::UNIX_EPOCH,
                state: vessel_proto::NodeState::Serving,
                public_key: None,
                timezone: None,
            };
            let lifecycle = crate::lifecycle::Lifecycle::new(
                info,
                stop.clone(),
                Arc::clone(&settings.state.runtime),
            )
            .with_settings(&settings);
            lifecycle.stop(when);
            assert!(lifecycle.is_draining());
            // WhenIdle has not polled its spawned waiter yet on this runtime.
            // The synchronous request itself must already close settings.
            // The real primitive's irreversible effect seal is tested beside it.
            assert_eq!(stop.is_cancelled(), matches!(when, StopWhen::Now));
            assert!(matches!(
                settings.admission.submit(recovery(RecoveryChoice::On)),
                Err(SettingsError::Stopping)
            ));
            phase("settings physical drain", settings.shutdown()).await;
        }
    })
    .await
    .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_create_past_lifecycle_precheck_cannot_launch_while_settings_drains() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path().join("data"));
        let home = root.path().join("home");
        let bin = root.path().join("bin");
        std::fs::create_dir(&home).unwrap();
        std::fs::create_dir(&bin).unwrap();
        let fake = vessel_testkit::Fake::Tool.path().unwrap();
        for role in [
            vessel_harness::fakes::Role::Structured,
            vessel_harness::fakes::Role::Terminal,
        ] {
            let _installed =
                vessel_harness::fakes::install(&bin, HarnessKind::ClaudeCode, role, &fake).unwrap();
        }
        let mut config =
            crate::Config::new(data.clone(), vessel_harness::HarnessEnv::new(home.clone()));
        config.harness_lookup = vessel_harness::Lookup::new([bin]);
        config.bind = Some("127.0.0.1:0".parse().unwrap());
        let node = Box::pin(phase("node starts", crate::Node::start(config)))
            .await
            .unwrap();
        let url = format!("ws://{}/rpc", node.address().unwrap())
            .parse()
            .unwrap();
        let bearer = data
            .load_or_create_token()
            .unwrap()
            .expose()
            .parse()
            .unwrap();
        let client = Box::pin(phase(
            "authenticated client connects",
            vessel_client::NodeClient::connect(
                &url,
                &bearer,
                vessel_proto::ClientInfo {
                    name: "admission-fixture".to_owned(),
                    version: "0".to_owned(),
                },
            ),
        ))
        .await
        .unwrap();
        let service = Arc::clone(&node.secure.service);
        let _stop_on_failure = StopOnDrop(service.0.lifecycle.clone());
        let settings = service.0.settings_for_test();
        let (write_entered, write_release) = settings.pause_write();
        let first = Arc::clone(&settings);
        let writer =
            tokio::spawn(async move { first.change(writer_tests::patch(ParkAfter::Never)).await });
        phase("enter real blocking writer", write_entered)
            .await
            .unwrap();
        let (place_entered, place_release) = service.0.place_gate.install();
        let create = tokio::spawn(async move {
            client
                .call(vessel_proto::CreateSession {
                    spec: vessel_proto::SessionSpec::new(
                        HarnessKind::ClaudeCode,
                        vessel_proto::SessionMode::Structured {
                            model: None,
                            resume: None,
                            options: Vec::new(),
                        },
                    )
                    .with_cwd(home.clone()),
                })
                .await
        });
        phase("create reaches post-precheck place", place_entered)
            .await
            .unwrap();
        service.0.lifecycle.stop(StopWhen::Now);
        let stopping = tokio::spawn(node.stop());
        assert!(settings.state.saved.try_lock().is_err());
        place_release.send(()).unwrap();
        assert!(
            phase("held create settles", create).await.unwrap().is_err(),
            "the request passed its lifecycle check but cannot start after stop"
        );
        assert!(settings.state.runtime.list().is_empty());
        write_release.send(()).unwrap();
        let _saved = phase("real writer settles", writer).await.unwrap().unwrap();
        phase("node physically stops", stopping)
            .await
            .unwrap()
            .unwrap();
        assert!(
            !root.path().join("home/fake-tool.log").exists(),
            "the actual fake was never launched"
        );
    })
    .await
    .unwrap();
}
