//! Real parent handles, placement and committed Begin exercise the SAME guard.
//! These tests do not spawn a child or qualify later revocation settlement.
use super::{AGENT_SCOPES, Agents, ServiceError, SessionId};
use crate::agents::creation_launch::Candidate;
use crate::agents::reservation::CountLaunchState;
use crate::agents::{CreationCredentials, CreationFenceRefusal, RejectedCreation};
use crate::agents::{ExecutionObservation, MAX_OWNED};
use crate::sessions::{CreationFixture, creation_fixture_phase};
use vessel_proto::Scope;
use vessel_proto::timeline::ExecutionInvalidation;

const TEST_BOUND: std::time::Duration = std::time::Duration::from_secs(120);

async fn case(work: impl AsyncFnOnce(&CreationFixture)) {
    vessel_testkit::within(
        TEST_BOUND,
        Box::pin(async {
            let fixture = CreationFixture::new().await;
            fixture.run(work(&fixture)).await;
        }),
    )
    .await
    .unwrap();
}

#[derive(Debug, PartialEq, Eq)]
enum RefusalKind {
    Registry,
    ParentChanged,
    ParentUnavailable,
    Grant,
    CountChanged,
}

fn refusal(reason: CreationFenceRefusal) -> RefusalKind {
    match reason {
        CreationFenceRefusal::Registry => RefusalKind::Registry,
        CreationFenceRefusal::ParentChanged => RefusalKind::ParentChanged,
        CreationFenceRefusal::ParentUnavailable => RefusalKind::ParentUnavailable,
        CreationFenceRefusal::Grant => RefusalKind::Grant,
        CreationFenceRefusal::CountChanged => RefusalKind::CountChanged,
        CreationFenceRefusal::Candidate(problem) => panic!("unexpected candidate: {problem:?}"),
        CreationFenceRefusal::Runtime(source) => panic!("unexpected runtime: {source}"),
        CreationFenceRefusal::Reservation(source) => panic!("unexpected reservation: {source}"),
    }
}

fn reservation_refusal<T>(result: Result<T, CreationFenceRefusal>) -> RefusalKind {
    match result {
        Ok(_unexpected) => panic!("reservation unexpectedly admitted"),
        Err(reason) => refusal(reason),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reservation_uses_original_registry_current_grant_and_incarnation() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let agents = fixture.agents();
        let parent = agents.capture_execution(&principal).unwrap();
        let child = SessionId::random();
        let count = agents
            .reserve_captured_creation(&parent, child, CreationCredentials::WithCredential)
            .unwrap();
        assert_eq!(agents.scopes(child), Some(AGENT_SCOPES.to_vec()));
        drop(count);
        assert!(!agents.lock().contains_key(&child));
        let foreign = Agents::new(agents.node);
        assert_eq!(
            reservation_refusal(foreign.reserve_captured_creation(
                &parent,
                child,
                CreationCredentials::WithoutCredential
            )),
            RefusalKind::Registry
        );
        agents.lock().get_mut(&parent.stamp.session).unwrap().scopes = vec![Scope::Read];
        assert_eq!(
            reservation_refusal(agents.reserve_captured_creation(
                &parent,
                child,
                CreationCredentials::WithoutCredential
            )),
            RefusalKind::Grant
        );
        agents.lock().get_mut(&parent.stamp.session).unwrap().scopes = AGENT_SCOPES.to_vec();
        agents.invalidate_execution(parent.stamp.session, ExecutionInvalidation::PersonStop);
        assert_eq!(
            reservation_refusal(agents.reserve_captured_creation(
                &parent,
                child,
                CreationCredentials::WithoutCredential
            )),
            RefusalKind::ParentChanged
        );
        assert!(!agents.lock().contains_key(&child));
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalidation_before_insertion_returns_the_original_candidate_and_begin() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let agents = fixture.agents();
        let capture = agents.capture_execution(&principal).unwrap();
        let child = SessionId::random();
        let mut count = agents
            .reserve_captured_creation(&capture, child, CreationCredentials::WithoutCredential)
            .unwrap();
        let input = fixture.input(capture, child).await;
        let begin = fixture.begin(&principal, child).await;
        count.begin(&begin).unwrap();
        let candidate = Candidate::prepare(input, &mut count, &begin).unwrap();
        agents.invalidate_execution(fixture.parent_id(), ExecutionInvalidation::Detach);
        let rejected: RejectedCreation<'_> = agents.admit_captured_creation(candidate).unwrap_err();
        assert_eq!(refusal(rejected.reason), RefusalKind::ParentChanged);
        assert_eq!(rejected.candidate.child(), child);
        assert_eq!(rejected.candidate.workspace(), fixture.workspace());
        let entries = agents.lock();
        assert!(
            rejected
                .candidate
                .count()
                .matches_pending(agents, entries.get(&child).unwrap())
        );
        drop(entries);
        assert_eq!(
            creation_fixture_phase(
                "retained Begin",
                fixture
                    .store()
                    .creation_operation(begin.snapshot().reserved.operation)
            )
            .await
            .unwrap(),
            Some(begin)
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_box_keeps_the_original_watch_and_count_is_consumed_once() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let agents = fixture.agents();
        let capture = agents.capture_execution(&principal).unwrap();
        let child = SessionId::random();
        let mut count = agents
            .reserve_captured_creation(&capture, child, CreationCredentials::WithoutCredential)
            .unwrap();
        let input = fixture.input(capture, child).await;
        let begin = fixture.begin(&principal, child).await;
        count.begin(&begin).unwrap();
        let candidate = Candidate::prepare(input, &mut count, &begin).unwrap();
        let mut tracked = agents.admit_captured_creation(candidate).unwrap();
        assert_eq!(tracked.observation(), ExecutionObservation::Current);
        assert_eq!(
            agents.lock().get(&child).unwrap().creation_launch,
            CountLaunchState::Admitted {
                operation: begin.snapshot().reserved.operation,
                revision: begin.snapshot().revision
            }
        );
        assert!(count.creation_correlation().is_err());
        agents.invalidate_execution(fixture.parent_id(), ExecutionInvalidation::Detach);
        assert_eq!(
            creation_fixture_phase("original admitted watch", tracked.invalidated()).await,
            ExecutionObservation::Invalidated(Some(ExecutionInvalidation::Detach))
        );
        drop(tracked);
        drop(count);
        assert!(
            agents.lock().contains_key(&child),
            "admitted settlement is never rollback on drop"
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_closure_refuses_before_count_admission_and_retains_its_cause() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let agents = fixture.agents();
        let capture = agents.capture_execution(&principal).unwrap();
        let child = SessionId::random();
        let mut count = agents
            .reserve_captured_creation(&capture, child, CreationCredentials::WithoutCredential)
            .unwrap();
        let input = fixture.input(capture, child).await;
        let begin = fixture.begin(&principal, child).await;
        count.begin(&begin).unwrap();
        let candidate = Candidate::prepare(input, &mut count, &begin).unwrap();
        fixture.close_runtime().await;
        let rejected = agents.admit_captured_creation(candidate).unwrap_err();
        assert!(matches!(
            rejected.reason,
            CreationFenceRefusal::Runtime(vessel_session::SessionError::ShuttingDown)
        ));
        let entries = agents.lock();
        assert!(
            rejected
                .candidate
                .count()
                .matches_pending(agents, entries.get(&child).unwrap())
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_replaced_child_entry_cannot_consume_the_old_begin_count_marker() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let agents = fixture.agents();
        let capture = agents.capture_execution(&principal).unwrap();
        let child = SessionId::random();
        let mut count = agents
            .reserve_captured_creation(&capture, child, CreationCredentials::WithoutCredential)
            .unwrap();
        let input = fixture.input(capture, child).await;
        let begin = fixture.begin(&principal, child).await;
        count.begin(&begin).unwrap();
        let candidate = Candidate::prepare(input, &mut count, &begin).unwrap();
        agents.forget(child);
        let replacement = agents.reserve_creation(child, &principal, false).unwrap();
        let rejected = agents.admit_captured_creation(candidate).unwrap_err();
        assert_eq!(refusal(rejected.reason), RefusalKind::CountChanged);
        assert_eq!(
            agents.lock().get(&child).unwrap().creation_launch,
            CountLaunchState::Reserved
        );
        drop(replacement);
        assert!(!agents.lock().contains_key(&child));
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_parent_count_uses_the_existing_last_slot_without_a_second_registry() {
    case(async |fixture| {
        let (principal, _connection) = fixture.parent().await;
        let agents = fixture.agents();
        let capture = agents.capture_execution(&principal).unwrap();
        let mut held = Vec::new();
        for _ in 0..MAX_OWNED {
            held.push(
                agents
                    .reserve_captured_creation(
                        &capture,
                        SessionId::random(),
                        CreationCredentials::WithoutCredential,
                    )
                    .unwrap(),
            );
        }
        let child = SessionId::random();
        assert!(matches!(
            agents.reserve_captured_creation(
                &capture,
                child,
                CreationCredentials::WithoutCredential
            ),
            Err(CreationFenceRefusal::Reservation(
                ServiceError::AgentLimit { limit: MAX_OWNED }
            ))
        ));
        assert!(!agents.lock().contains_key(&child));
        drop(held);
        assert_eq!(agents.lock().len(), 1);
        assert_eq!(capture.observation(), ExecutionObservation::Current);
    })
    .await;
}
