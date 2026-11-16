//! Real local placement and committed Begin qualify constructor inputs only.
use super::*;
use crate::sessions::CreationFixture;
use futures_util::FutureExt as _;
use std::panic::AssertUnwindSafe;

const TEST_BOUND: Duration = Duration::from_secs(120);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn binding_refusal_retains_exact_input_without_consuming_count_correlation() {
    vessel_testkit::within(
        TEST_BOUND,
        Box::pin(async {
            let fixture = CreationFixture::new().await;
            fixture
                .run(async {
                    let (principal, _connection) = fixture.parent().await;
                    let agents = fixture.agents();
                    let capture = agents.capture_execution(&principal).unwrap();
                    let stamp = capture.stamp();
                    let child = SessionId::random();
                    let mut count = agents.reserve_creation(child, &principal, false).unwrap();
                    let mut input = fixture.input(capture, child).await;
                    let begin = fixture.begin(&principal, child).await;
                    count.begin(&begin).unwrap();
                    input.options.id = Some(SessionId::random());
                    let rejected = Candidate::prepare(input, &mut count, &begin).unwrap_err();
                    assert_eq!(rejected.problem, CandidateProblem::Binding);
                    assert_eq!(rejected.input.parent.stamp(), stamp);
                    assert_eq!(rejected.input.placement.child(), child);
                    assert_eq!(rejected.input.placement.workspace(), fixture.workspace());
                    let mut retained = *rejected.input;
                    retained.options.id = Some(child);
                    // The failed local preparation did not consume the single-use
                    // relation; retry corrects only prepared input, never spawns.
                    let candidate = Candidate::prepare(retained, &mut count, &begin).unwrap();
                    assert_eq!(candidate.child(), child);
                    assert!(count.creation_correlation().is_err());
                })
                .await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn candidate_uses_actual_project_metadata_start_and_committed_begin_owners() {
    vessel_testkit::within(
        TEST_BOUND,
        Box::pin(async {
            let fixture = CreationFixture::new().await;
            fixture
                .run(async {
                    let (principal, _connection) = fixture.parent().await;
                    let agents = fixture.agents();
                    let capture = agents.capture_execution(&principal).unwrap();
                    let child = SessionId::random();
                    let mut count = agents.reserve_creation(child, &principal, false).unwrap();
                    let input = fixture.input(capture, child).await;
                    let begin = fixture.begin(&principal, child).await;
                    count.begin(&begin).unwrap();
                    let candidate = Candidate::prepare(input, &mut count, &begin).unwrap();
                    assert_eq!(candidate.child(), child);
                    assert_eq!(candidate.workspace(), fixture.workspace());
                    assert!(candidate.owner_matches(agents));
                    assert!(candidate.check_deadline().is_ok());
                    assert!(candidate.check_runtime().is_ok());
                    assert_eq!(
                        candidate.capture().observation(),
                        ExecutionObservation::Current
                    );
                    let stored = crate::sessions::creation_fixture_phase(
                        "Begin reread",
                        fixture
                            .store()
                            .creation_operation(begin.snapshot().reserved.operation),
                    )
                    .await
                    .unwrap()
                    .unwrap();
                    assert_eq!(stored, begin);
                    let entries = agents.lock();
                    assert!(
                        candidate
                            .count()
                            .matches_pending(agents, entries.get(&child).unwrap())
                    );
                    drop(entries);
                    fixture.close_runtime().await;
                    assert!(matches!(
                        candidate.check_runtime(),
                        Err(vessel_session::SessionError::ShuttingDown)
                    ));
                })
                .await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assertion_failure_still_closes_actual_parent_and_store_before_resuming() {
    vessel_testkit::within(
        TEST_BOUND,
        Box::pin(async {
            let fixture = CreationFixture::new().await;
            let failure = AssertUnwindSafe(fixture.run(async {
                let (_principal, _connection) = fixture.parent().await;
                panic!("intentional fixture assertion failure");
            }))
            .catch_unwind()
            .await;
            assert!(failure.is_err());
            assert!(fixture.agents().scopes(fixture.parent_id()).is_none());
            assert!(matches!(
                crate::sessions::creation_fixture_phase(
                    "closed writer refusal",
                    fixture.store().advance_creation(
                        vessel_proto::WorkspaceOperationId::random(),
                        vessel_proto::creation::Revision::INITIAL,
                        vessel_proto::creation::Advance::Begin(
                            vessel_proto::creation::Stage::Resource
                        ),
                    )
                )
                .await,
                Err(vessel_store::StoreError::Closed)
            ));
        }),
    )
    .await
    .unwrap();
}
