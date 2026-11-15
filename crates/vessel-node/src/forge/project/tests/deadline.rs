//! Test safety after fake preparation, with named phases for stuck owners.
//! The expiry case adds only its deliberate virtual advance to the outer
//! budget; I/O phases and the original root completion bound stay five seconds.
use std::{future::Future, time::Duration};

pub(in crate::forge::project) const LIMIT: Duration = Duration::from_secs(10);
pub(in crate::forge::project) const PHASE: Duration = Duration::from_secs(5);

pub(in crate::forge::project) fn case<F: Future<Output = ()>>(
    limit: Duration,
    future: F,
) -> impl Future<Output = ()> {
    let future = Box::pin(future);
    async move {
        vessel_testkit::within(limit, future)
            .await
            .expect("the project fixture must finish within its safety budget");
    }
}

pub(in crate::forge::project) fn step<F: Future>(
    name: &'static str,
    future: F,
) -> impl Future<Output = F::Output> {
    let future = Box::pin(future);
    async move {
        tokio::time::timeout(PHASE, future)
            .await
            .unwrap_or_else(|elapsed| panic!("project fixture phase {name}: {elapsed}"))
    }
}
