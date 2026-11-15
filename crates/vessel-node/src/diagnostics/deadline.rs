//! Deadlines for staged diagnostic fixtures; no product admission policy.
//! A failed barrier must name its phase instead of hanging the whole gate.
use std::{future::Future, time::Duration};

const SAFETY_NET: Duration = Duration::from_secs(60);
const PHASE_DEADLINE: Duration = Duration::from_secs(20);

pub(super) fn run(test: impl Future<Output = ()>) -> impl Future<Output = ()> {
    // Box before constructing the timeout future, like the existing e2e step
    // helper, so nested Store futures do not inflate the test task's stack.
    let test = Box::pin(test);
    async move {
        vessel_testkit::within(SAFETY_NET, test)
            .await
            .unwrap_or_else(|error| panic!("diagnostic fixture exceeded {SAFETY_NET:?}: {error}"));
    }
}
pub(super) fn phase<F: Future>(name: &str, work: F) -> impl Future<Output = F::Output> {
    let work = Box::pin(work);
    async move {
        tokio::time::timeout(PHASE_DEADLINE, work)
            .await
            .unwrap_or_else(|error| {
                panic!("diagnostic phase {name} exceeded {PHASE_DEADLINE:?}: {error}")
            })
    }
}
