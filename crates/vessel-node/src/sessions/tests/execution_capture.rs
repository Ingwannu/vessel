//! Staged capture tests use actual registered fake `SessionHandle`s. They do not
//! qualify parentage, queue insertion, persistence or a private notice capability.
use super::*;
use crate::agents::{Agents, CaptureRefusal, CapturedParentExecution, ExecutionObservation};
use crate::principal::Principal;
use futures_util::FutureExt;
use std::panic::AssertUnwindSafe;
use std::time::Duration;
mod owner;
use owner::{OwnedFixture, Phase, step};
use vessel_proto::timeline::{ExecutionInvalidation, FinishGrantCeiling};
use vessel_proto::{DeviceId, NodeId, Origin, OriginBy, Scope};

const CAPTURE_TEST_BOUND: Duration = Duration::from_secs(120);

fn principal(node: NodeId, session: SessionId, scopes: Vec<Scope>) -> Principal {
    Principal::Agent {
        node,
        session,
        scopes,
    }
}

async fn live(fixture: &Fixture) -> (SessionId, vessel_session::SessionHandle) {
    let info = step(
        Phase::Create,
        fixture
            .sessions
            .create(fixture.structured(), LaunchOptions::default(), None),
    )
    .await
    .unwrap();
    let Found::Live { handle, .. } = step(Phase::Find, fixture.sessions.find(info.id))
        .await
        .unwrap()
    else {
        panic!("live fake");
    };
    let _running = step(Phase::Ready, ready(&handle)).await.unwrap();
    (info.id, handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_direct_local_agent_capture_keeps_the_original_runtime_and_grant_ceiling() {
    vessel_testkit::within(CAPTURE_TEST_BOUND, async {
        let fixture = OwnedFixture::new().await;
        let outcome = AssertUnwindSafe(check_capture(fixture.get()))
            .catch_unwind()
            .await;
        fixture.finish().await;
        if let Err(payload) = outcome {
            std::panic::resume_unwind(payload);
        }
    })
    .await
    .unwrap();
}

// One causal chain must compare the same original registry/watch before and
// after invalidation, replacement capture and final handle shutdown.
async fn check_capture(fixture: &Fixture) {
    let (session, handle) = live(fixture).await;
    let node = NodeId::random();
    let agents = Arc::new(Agents::new(node));
    let both = vec![Scope::Read, Scope::Operate];
    let local = principal(node, session, both.clone());
    let _secret = agents.reserve(session, &Principal::Owner, false).unwrap();
    assert!(matches!(
        agents.capture_execution(&local),
        Err(CaptureRefusal::Unavailable)
    ));
    agents.started(session, handle.clone());
    check_principal_refusals(&agents, node, session, &both);
    let limited = Arc::new(Agents::new(node));
    let _reserved = limited
        .reserve(
            session,
            &Principal::Device {
                id: DeviceId::random(),
                scopes: vec![Scope::Read],
            },
            false,
        )
        .unwrap();
    limited.started(session, handle.clone());
    assert!(matches!(
        limited.capture_execution(&local),
        Err(CaptureRefusal::Grant)
    ));
    let mut captured: CapturedParentExecution = agents.capture_execution(&local).unwrap();
    assert_eq!(captured.stamp().node, node);
    assert_eq!(captured.stamp().session, session);
    assert_eq!(captured.ceiling(), FinishGrantCeiling::ReadOperate);
    assert_eq!(captured.observation(), ExecutionObservation::Current);
    agents.invalidate_execution(session, ExecutionInvalidation::PersonStop);
    assert_eq!(
        captured.observation(),
        ExecutionObservation::Invalidated(Some(ExecutionInvalidation::PersonStop))
    );
    assert_eq!(
        step(Phase::Invalidation, captured.invalidated()).await,
        ExecutionObservation::Invalidated(Some(ExecutionInvalidation::PersonStop))
    );
    let mut fresh = agents.capture_execution(&local).unwrap();
    assert_ne!(captured.stamp().incarnation, fresh.stamp().incarnation);
    // A later fresh capture never replaces authority in the old object.
    assert_eq!(captured.ceiling(), FinishGrantCeiling::ReadOperate);
    assert_eq!(
        captured.observation(),
        ExecutionObservation::Invalidated(Some(ExecutionInvalidation::PersonStop))
    );
    agents.retire(session);
    assert_eq!(fresh.observation(), ExecutionObservation::Unavailable);
    assert_eq!(
        step(Phase::RetiredWatch, fresh.invalidated()).await,
        ExecutionObservation::Unavailable
    );
    step(Phase::EndedHandle, fixture.runtime.shutdown()).await;
    // Ended is refused even while another registry still retains its entry.
    assert!(matches!(
        limited.capture_execution(&local),
        Err(CaptureRefusal::Unavailable)
    ));
}

#[derive(Debug)]
struct AssertionProbe;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_assertion_failure_still_drains_the_actual_fake_fixture() {
    vessel_testkit::within(CAPTURE_TEST_BOUND, async {
        let fixture = OwnedFixture::new().await;
        let mut handle = None;
        let outcome: Result<(), _> = AssertUnwindSafe(async {
            let (_, live) = live(fixture.get()).await;
            handle = Some(live);
            std::panic::panic_any(AssertionProbe);
        })
        .catch_unwind()
        .await;
        fixture.finish().await;
        assert!(outcome.is_err_and(|payload| payload.is::<AssertionProbe>()));
        assert!(handle.unwrap().has_ended());
    })
    .await
    .unwrap();
}

fn check_principal_refusals(
    agents: &Arc<Agents>,
    node: NodeId,
    session: SessionId,
    both: &[Scope],
) {
    for unsupported in [
        Principal::Owner,
        Principal::Device {
            id: DeviceId::random(),
            scopes: both.to_vec(),
        },
        Principal::delegated(
            DeviceId::random(),
            &both,
            Origin {
                node,
                by: OriginBy::Agent { session, depth: 0 },
                scopes: both.to_vec(),
            },
            NodeId::random(),
        ),
    ] {
        assert!(matches!(
            agents.capture_execution(&unsupported),
            Err(CaptureRefusal::UnsupportedPrincipal)
        ));
    }
    assert!(matches!(
        agents.capture_execution(&principal(NodeId::random(), session, both.to_vec())),
        Err(CaptureRefusal::ForeignNode)
    ));
    assert!(matches!(
        agents.capture_execution(&principal(node, session, vec![Scope::Read])),
        Err(CaptureRefusal::Grant)
    ));
}
