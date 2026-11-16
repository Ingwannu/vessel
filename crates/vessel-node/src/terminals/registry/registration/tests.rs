use super::*;
use std::time::Duration;

const SAFETY_NET: Duration = Duration::from_secs(60);
const PHASE_WAIT: Duration = Duration::from_secs(15);

#[tokio::test]
async fn node_shutdown_bounds_a_missing_end_observation_without_calling_it_success() {
    vessel_testkit::within(SAFETY_NET, async {
        let shutdown = CancellationToken::new();
        shutdown.cancel();
        let observed =
            tokio::time::timeout(PHASE_WAIT, observe_end(std::future::pending(), shutdown))
                .await
                .expect("shutdown end-observation phase escaped its finite bound");
        assert_eq!(observed, EndObservation::ShutdownUnconfirmed);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn an_already_observed_end_remains_observed_when_shutdown_is_also_ready() {
    vessel_testkit::within(SAFETY_NET, async {
        let shutdown = CancellationToken::new();
        shutdown.cancel();
        let observed = tokio::time::timeout(PHASE_WAIT, observe_end(async {}, shutdown))
            .await
            .expect("ready end-observation phase did not finish");
        assert_eq!(observed, EndObservation::Observed);
    })
    .await
    .unwrap();
}
