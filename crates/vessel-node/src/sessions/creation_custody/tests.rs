//! Actual child drain exercises watch custody, not creation launch permission.
use super::*;
use crate::sessions::{CreationFixture, creation_fixture_phase as phase};
use std::time::Duration;
use vessel_proto::{SessionStatus, timeline::ExecutionInvalidation};

const CUSTODY_BOUND: Duration = Duration::from_secs(120);

async fn case(body: impl AsyncFnOnce(&CreationFixture)) {
    vessel_testkit::within(
        CUSTODY_BOUND,
        Box::pin(async {
            let fixture = CreationFixture::new().await;
            fixture.run(body(&fixture)).await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalidation_before_custody_first_poll_stops_the_actual_created_child() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let capture = fixture.agents().capture_execution(&principal).unwrap();
        let child = SessionId::random();
        let _secret = fixture.agents().reserve(child, &principal, false).unwrap();
        fixture
            .agents()
            .invalidate_execution(fixture.parent_id(), ExecutionInvalidation::Detach);
        let registered = phase(
            "actual child registration",
            fixture.start_child_custody(capture, child),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            registered.parent_at_registration,
            ExecutionObservation::Invalidated(Some(ExecutionInvalidation::Detach))
        );
        let ended = phase("revoked child ended", registered.handle.ended()).await;
        assert!(matches!(ended.status, SessionStatus::Exited { .. }));
        let stored = fixture.child_stored(child).await;
        assert_eq!(stored.id, child);
        assert!(matches!(stored.status, SessionStatus::Exited { .. }));
        assert!(!fixture.agents().is_live(child));
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalidation_after_registration_is_owned_by_the_same_child_persistence_task() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let capture = fixture.agents().capture_execution(&principal).unwrap();
        let child = SessionId::random();
        let _secret = fixture.agents().reserve(child, &principal, false).unwrap();
        let registered = phase(
            "actual child registration",
            fixture.start_child_custody(capture, child),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            registered.parent_at_registration,
            ExecutionObservation::Current
        );
        assert!(!registered.handle.has_ended());
        fixture
            .agents()
            .invalidate_execution(fixture.parent_id(), ExecutionInvalidation::Archive);
        let _ended = phase("lifecycle child stop", registered.handle.ended()).await;
        let stored = fixture.child_stored(child).await;
        assert!(matches!(stored.status, SessionStatus::Exited { .. }));
        assert!(!fixture.agents().is_live(child));
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normal_child_completion_does_not_wait_for_parent_invalidation() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let capture = fixture.agents().capture_execution(&principal).unwrap();
        let child = SessionId::random();
        let _secret = fixture.agents().reserve(child, &principal, false).unwrap();
        let registered = phase(
            "actual child registration",
            fixture.start_child_custody(capture, child),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            registered.parent_at_registration,
            ExecutionObservation::Current
        );
        registered.handle.kill();
        let _ended = phase("normal child stop", registered.handle.ended()).await;
        let stored = fixture.child_stored(child).await;
        assert!(matches!(stored.status, SessionStatus::Exited { .. }));
        assert!(fixture.agents().is_live(fixture.parent_id()));
    })
    .await;
}
