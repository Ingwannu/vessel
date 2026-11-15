//! Physical settlement precedes runtime/Store close; drain seals effects at once.

use std::sync::Arc;
use std::task::Poll;

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::restart::SetRestartRecovery;
use vessel_proto::settings::SetNodeSettings;
use vessel_proto::{ErrorCode, StopWhen};

use super::writer_tests::{WRITE_TEST_BUDGET, runtime_with_stop};
use super::*;

fn recovery(enabled: bool) -> SetNodeSettings {
    SetNodeSettings {
        restart_recovery: Some(SetRestartRecovery {
            enabled: Some(enabled),
            prompt: None,
        }),
        ..SetNodeSettings::default()
    }
}

#[tokio::test]
async fn teardown_drains_saved_off_and_queued_on_before_runtime_and_store_close() {
    tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let runtime_stop = CancellationToken::new();
        let settings = Arc::new(
            Settings::open(
                DataDir::new(root.path()),
                runtime_with_stop(root.path(), runtime_stop.clone()),
                ParkAfter::DEFAULT,
                false,
            )
            .await,
        );
        settings.change(recovery(true)).await.unwrap();
        let store = settings.state.recovery.store();
        let (entered, release) = settings.pause_cleanup();
        let first = Arc::clone(&settings);
        let caller = tokio::spawn(async move { first.change(recovery(false)).await });
        entered.await.unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        let saved_off = settings
            .state
            .data_dir
            .read_settings()
            .unwrap()
            .unwrap()
            .public_settings();
        assert!(!saved_off.restart_recovery.unwrap().effective().enabled);
        assert!(settings.state.saved.try_lock().is_err());
        assert!(!settings.policy().snapshot().0.enabled);
        let mut queued_on = Box::pin(settings.change(recovery(true)));
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
        assert!(store.sessions().await.is_ok());
        let (_, generation) = settings.policy().snapshot();
        assert_eq!(
            settings.policy().check(generation).unwrap_err().code,
            ErrorCode::Unavailable
        );
        release.send(()).unwrap();
        assert!(queued_on.await.unwrap().restart_recovery.enabled);
        stop.await;
        assert!(runtime_stop.is_cancelled());
        assert!(matches!(
            store.sessions().await,
            Err(vessel_store::StoreError::Closed)
        ));
        let (_, generation) = settings.policy().snapshot();
        assert_eq!(
            settings.policy().check(generation).unwrap_err().code,
            ErrorCode::Unavailable
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn now_and_drain_seal_the_settings_and_effect_owners_synchronously() {
    tokio::time::timeout(WRITE_TEST_BUDGET, async {
        for when in [StopWhen::Now, StopWhen::WhenIdle] {
            let root = tempfile::tempdir().unwrap();
            let settings = super::writer_tests::settings(root.path()).await;
            settings.change(recovery(true)).await.unwrap();
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
            // The synchronous request itself must already have sealed effects.
            assert_eq!(stop.is_cancelled(), matches!(when, StopWhen::Now));
            let (_, generation) = settings.policy().snapshot();
            assert_eq!(
                settings.policy().check(generation).unwrap_err().code,
                ErrorCode::Unavailable
            );
            assert!(matches!(
                settings.admission.submit(recovery(true)),
                Err(SettingsError::Stopping)
            ));
            settings.shutdown().await;
        }
    })
    .await
    .unwrap();
}
