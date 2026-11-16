//! Test ownership survives assertion panic long enough to request actual stop/drain.
//! The validation task is not the runner: aborting it never certifies cleanup.
use super::{Fixture, within};
use std::{future::Future, sync::Arc, time::Duration};
const BODY: Duration = Duration::from_secs(35);
const DRAIN: Duration = Duration::from_secs(10);
const VALIDATION_JOIN: Duration = Duration::from_secs(5);

pub(super) async fn case<F>(body: impl FnOnce(Arc<Fixture>) -> F)
where
    F: Future<Output = ()> + Send + 'static,
{
    within(async {
        let fixture = Arc::new(Fixture::new());
        // Join captures assertion panics; original fixture ownership remains here.
        let mut validation = tokio::spawn(body(Arc::clone(&fixture)));
        let outcome = tokio::time::timeout(BODY, &mut validation).await;
        fixture.shutdown.cancel();
        let _closed = fixture.tasks.close();
        if outcome.is_err() {
            validation.abort();
            let _cancelled_validation = tokio::time::timeout(VALIDATION_JOIN, &mut validation)
                .await.expect("validation task did not acknowledge cancellation");
        }
        tokio::time::timeout(DRAIN, fixture.tasks.wait()).await
            .expect("actual terminal tracker drain timed out after test stop request; runner is NOT settled");
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(error)) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
            Ok(Err(error)) => panic!("cleanup validation task cancelled: {error}"),
            Err(_deadline) => panic!("cleanup validation body timed out before actual stop/drain"),
        }
    }).await;
}
