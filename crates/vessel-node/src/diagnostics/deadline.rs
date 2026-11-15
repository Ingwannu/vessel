//! Deadlines for staged diagnostic fixtures; no product admission policy.
//! A failed barrier must name its phase instead of hanging the whole gate.
use std::{future::Future, time::Duration};

const SAFETY_NET: Duration = Duration::from_secs(60);
const PHASE_DEADLINE: Duration = Duration::from_secs(20);

pub(super) async fn run(test: impl Future<Output = ()>) {
    vessel_testkit::within(SAFETY_NET, test)
        .await
        .unwrap_or_else(|error| panic!("diagnostic fixture exceeded {SAFETY_NET:?}: {error}"));
}
pub(super) async fn phase<F: Future>(name: &str, work: F) -> F::Output {
    tokio::time::timeout(PHASE_DEADLINE, work)
        .await
        .unwrap_or_else(|error| {
            panic!("diagnostic phase {name} exceeded {PHASE_DEADLINE:?}: {error}")
        })
}
