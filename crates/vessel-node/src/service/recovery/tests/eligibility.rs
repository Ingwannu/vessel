//! Imported display metadata must not turn a delegated creator into a local one.
use super::*;
use crate::principal::Principal;
use vessel_proto::{DeviceId, NodeId, Origin, OriginBy, Scope, SessionOrigin};

async fn imported(
    fixture: &Fixture,
    node: &Node,
    owner: &NodeClient,
    principal: &Principal,
) -> SessionId {
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(fixture.work.clone())
    .with_profile("recovery".parse().unwrap());
    let info = node
        .secure
        .service
        .0
        .create_as(
            principal,
            spec,
            Some(SessionOrigin::Imported {
                node: NodeId::random(),
                session: SessionId::random(),
            }),
            None,
        )
        .await
        .unwrap();
    assert!(matches!(info.origin, Some(SessionOrigin::Imported { .. })));
    let mut stream = owner.follow(info.id, EventSeq::ZERO).await.unwrap();
    loop {
        let record = stream.next().await.unwrap().unwrap();
        if matches!(
            record.event,
            SessionEvent::Status {
                status: SessionStatus::Running
            }
        ) {
            break;
        }
    }
    assert_eq!(
        owner
            .prompt(
                info.id,
                vec![ContentBlock::Text {
                    text: "wait".into()
                }],
                Delivery::Start
            )
            .await
            .unwrap(),
        vessel_proto::Delivered::Started
    );
    loop {
        let record = stream.next().await.unwrap().unwrap();
        if matches!(
            record.event,
            SessionEvent::Agent {
                event: AgentEvent::TurnStarted { .. }
            }
        ) {
            break;
        }
    }
    let kept = owner
        .read(info.id, EventSeq::ZERO, EVENT_PAGE)
        .await
        .unwrap();
    assert!(kept.records.iter().any(|record| matches!(
        record.event,
        SessionEvent::Agent {
            event: AgentEvent::TurnStarted { .. }
        }
    )));
    info.id
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_delegated_import_stays_excluded_while_a_local_import_can_resume_after_a_local_turn() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let fixture = Fixture::new();
            let node = fixture.start().await;
            let owner = fixture.client(&node).await;
            set(&node, true).await;
            let delegated = Principal::delegated(
                DeviceId::random(),
                &[Scope::Read, Scope::Operate],
                Origin {
                    node: NodeId::random(),
                    by: OriginBy::Owner,
                    scopes: vec![Scope::Read, Scope::Operate],
                },
                node.info().id,
            );
            let local = imported(&fixture, &node, &owner, &Principal::Owner).await;
            let remote = imported(&fixture, &node, &owner, &delegated).await;
            node.stop().await.unwrap();
            // Open only the real projection: no coordinator consumes a wrongly
            // classified intent and hides the regression behind a later grant
            // lookup. Transfer/file-copy RPC authority is a separate concern.
            let store = vessel_store::Store::open_with_restart_policy(
                &fixture.data,
                vessel_store::StoreConfig::default(),
                true,
            )
            .await
            .unwrap()
            .store;
            let candidate = store.next_restart().await.unwrap().unwrap();
            assert_eq!(candidate.session, local);
            assert_ne!(candidate.session, remote);
            assert!(store.claim_restart(candidate).await.unwrap());
            store.finish_restart(local).await.unwrap();
            assert!(store.next_restart().await.unwrap().is_none());
            store.close().await;
        }),
    )
    .await
    .unwrap();
}
