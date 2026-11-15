//! Named finite waits for the new provider/credential fixtures. Whole
//! cases prepare fakes first; phase preparation then uses that cached result.
use std::{future::Future, time::Duration};

pub(crate) fn step<F: Future>(name: &'static str, future: F) -> impl Future<Output = F::Output> {
    let future = Box::pin(future);
    async move {
        vessel_testkit::within(Duration::from_secs(5), future)
            .await
            .unwrap_or_else(|elapsed| panic!("forge fixture phase {name}: {elapsed}"))
    }
}
