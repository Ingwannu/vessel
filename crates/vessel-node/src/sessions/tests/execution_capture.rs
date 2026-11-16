//! Staged capture tests use actual registered fake `SessionHandle`s. They do not
//! qualify parentage, queue insertion, persistence or a private notice capability.
use super::*;
use crate::agents::{Agents, CaptureRefusal, CapturedParentExecution, ExecutionObservation};
use crate::principal::Principal;
use std::time::Duration;
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
    let info = fixture
        .sessions
        .create(fixture.structured(), LaunchOptions::default(), None)
        .await
        .unwrap();
    let Found::Live { handle, .. } = fixture.sessions.find(info.id).await.unwrap() else {
        panic!("live fake");
    };
    let _running = ready(&handle).await.unwrap();
    (info.id, handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_direct_local_agent_capture_keeps_the_original_runtime_and_grant_ceiling() {
    vessel_testkit::within(CAPTURE_TEST_BOUND, async {
        let fixture = Fixture::new().await;
        let (session, handle) = live(&fixture).await;
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
        for unsupported in [
            Principal::Owner,
            Principal::Device {
                id: DeviceId::random(),
                scopes: both.clone(),
            },
            Principal::delegated(
                DeviceId::random(),
                &both,
                Origin {
                    node,
                    by: OriginBy::Agent { session, depth: 0 },
                    scopes: both.clone(),
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
            agents.capture_execution(&principal(NodeId::random(), session, both.clone())),
            Err(CaptureRefusal::ForeignNode)
        ));
        assert!(matches!(
            agents.capture_execution(&principal(node, session, vec![Scope::Read])),
            Err(CaptureRefusal::Grant)
        ));
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
            captured.invalidated().await,
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
        assert_eq!(fresh.invalidated().await, ExecutionObservation::Unavailable);
        fixture.runtime.shutdown().await;
        // Ended is refused even while another registry still retains its entry.
        assert!(matches!(
            limited.capture_execution(&local),
            Err(CaptureRefusal::Unavailable)
        ));
        fixture.settle().await;
        fixture.store.close().await;
    })
    .await
    .unwrap();
}
