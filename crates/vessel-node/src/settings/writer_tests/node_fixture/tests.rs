//! Force the same fixture's failure paths, then inspect actual settled owners.

use super::*;
use crate::settings::writer_tests::{WRITE_PHASE_BUDGET, WRITE_TEST_BUDGET, patch, phase};
use vessel_proto::parking::ParkAfter;
use vessel_store::DataDir;

fn config(data: &DataDir, root: &std::path::Path) -> crate::Config {
    let mut config = crate::Config::new(
        data.clone(),
        vessel_harness::HarnessEnv::new(root.join("home")),
    );
    config.bind = Some("127.0.0.1:0".parse().unwrap());
    config.harness_lookup = vessel_harness::Lookup::new(Vec::new());
    config
}

#[tokio::test]
async fn failed_setup_is_rethrown_only_after_the_original_node_is_drained() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("data"));
    let running = Running::start(
        config(&data, root.path()),
        Budget::new(WRITE_TEST_BUDGET, WRITE_PHASE_BUDGET),
        |_owned| async { panic!("deliberate fixture setup assertion") },
    );
    let settled = running.finish().await;
    phase(
        "failed setup's actual node and Store drain",
        settled.assert_drained(),
    )
    .await;
    settled.assert_observed();
    assert!(settled.body.is_err());
    let original = std::panic::catch_unwind(AssertUnwindSafe(|| settled.assert_success()));
    assert!(
        original.is_err(),
        "cleanup must not replace the original failure with success"
    );
}

#[tokio::test]
async fn lost_body_observer_releases_the_real_gate_and_joins_its_writer_and_node() {
    exercise_observer_loss(Observer::Dropped).await;
}

#[tokio::test]
async fn expired_case_observer_keeps_the_driver_until_actual_cleanup_finishes() {
    exercise_observer_loss(Observer::Expired).await;
}

#[derive(Clone, Copy)]
enum Observer {
    Dropped,
    Expired,
}

async fn exercise_observer_loss(observer: Observer) {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("data"));
    let (ready, entered) = oneshot::channel();
    let (applied, written) = oneshot::channel();
    let mut running = Running::start(
        config(&data, root.path()),
        Budget::new(WRITE_TEST_BUDGET, WRITE_PHASE_BUDGET),
        move |owned| async move {
            let settings = owned.service.0.settings_for_test();
            let (writing, release) = settings.pause_write();
            owned.retain(Gate::Write, release);
            drop(owned.tasks.spawn(async move {
                let result = settings.change(patch(ParkAfter::Never)).await;
                applied.send(result).unwrap();
            }));
            phase("failure control enters real blocking writer", writing)
                .await
                .unwrap();
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
        },
    );
    // A failed phase still triggers retained cleanup before its original error
    // is raised. Do not unwrap this observation while a real Node is owned.
    let reached = tokio::time::timeout(WRITE_PHASE_BUDGET, entered).await;
    match observer {
        Observer::Dropped => running.abandon_observer(),
        Observer::Expired => {
            // Expire only the outer observer's recorded clock. The driver
            // retains its original startup/body clocks and real held writer.
            running.started = running.started.checked_sub(WRITE_TEST_BUDGET).unwrap();
        }
    }
    let settled = running.finish().await;
    phase(
        "lost observer's actual node and Store drain",
        settled.assert_drained(),
    )
    .await;
    reached.unwrap().unwrap();
    assert!(matches!(settled.body, Ok(Body::ObserverLost)));
    assert_eq!(
        settled.case_expired.is_some(),
        matches!(observer, Observer::Expired)
    );
    assert_eq!(
        phase("actual retained writer acknowledgement", written)
            .await
            .unwrap()
            .unwrap()
            .park_after,
        ParkAfter::Never
    );
    assert_eq!(
        data.read_settings()
            .unwrap()
            .unwrap()
            .public_settings()
            .park_after,
        Some(ParkAfter::Never)
    );
}
