//! Typed payloads from a real framed subscription, plus canonical attempt/ticket ingress.
//! These are component tests, not real App receive/input or resident product proof.
use super::*;
use crate::{
    State,
    testing::{Home, peer, phase, state, within},
};
use vessel_app_view::{
    layout::{AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Target},
    residents::{PanelClaim, Residents, SessionAddress},
};
use vessel_proto::{
    EventSeq, ServerFrame, SessionEvent, SessionId, SessionStatus, SubscriptionEnd, SubscriptionId,
    WorkspaceId,
};
const QUEUED_PARTS: usize = 3;

struct Fixture {
    _home: Home,
    state: State,
    peer: peer::Peer,
    selected: SelectedConnection,
    context: ConnectionContext,
    residents: Residents,
    binding: FollowBinding,
}
async fn fixture() -> Fixture {
    let home = Home::new();
    let (mut state, _events) = state(&home);
    let mut peer = peer::connect("fixture").await;
    let client = peer.client.take().unwrap();
    let entry = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: Some(client.node().id),
    };
    let connected = state
        .admit_connection("fixture", &entry, client, Vec::new())
        .unwrap();
    let context = connected.context.clone();
    let _old = state.clients.insert("fixture".to_owned(), connected);
    let selected = state.selected_connection(&context).unwrap();
    let address = SessionAddress {
        node: context.node(),
        session: SessionId::random(),
    };
    let key = LayoutKey::Workspace {
        node: address.node,
        workspace: WorkspaceId::random(),
    };
    let mut layout = Layout::default();
    let tab = layout
        .open(
            Target::Agent(address.session),
            OpenIntent::New,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap();
    let mut residents = Residents::new();
    let token = residents
        .claim(
            address,
            PanelClaim {
                workspace: key,
                tab,
            },
            PaneId::MAIN,
        )
        .unwrap();
    let binding = residents.begin_follow(token).unwrap();
    Fixture {
        _home: home,
        state,
        peer,
        selected,
        context,
        residents,
        binding,
    }
}

fn install(
    residents: &mut Residents,
    envelope: Envelope,
    count: &mut usize,
) -> Result<(), Problem> {
    envelope.install(|binding, payload| {
        match payload {
            Payload::Completion { operation, result } => {
                residents.settle_operation(binding, operation.kind, operation.ticket)?;
                assert!(result.is_ok());
            }
            Payload::Metadata(_)
            | Payload::Conversation(_)
            | Payload::Records(_)
            | Payload::Lost(_)
            | Payload::Eof => residents.validate_binding(binding)?,
        }
        *count += 1;
        Ok(())
    })
}

#[tokio::test]
async fn queued_real_metadata_records_and_terminal_frames_recheck_the_original_lease() {
    within(async {
        for end in [SubscriptionEnd::SessionEnded, SubscriptionEnd::Unsubscribed] {
            let fixture = fixture().await;
            let scope =
                Scope::capture(&fixture.selected, &fixture.context, fixture.binding).unwrap();
            let session = fixture.binding.resident.address.session;
            let mut follow = phase(
                "attach real framed session",
                fixture.selected.follow_session(session, EventSeq::ZERO),
            )
            .await
            .unwrap();
            let (sender, mut receiving) = tokio::sync::mpsc::channel(QUEUED_PARTS);
            phase(
                "queue admitted metadata",
                fixture.selected.send_guarded(
                    &sender,
                    scope.envelope(Payload::Metadata(Box::new(follow.info().unwrap().clone()))),
                ),
            )
            .await
            .unwrap();
            fixture
                .peer
                .push(ServerFrame::Event {
                    subscription: SubscriptionId::new(1),
                    record: EventRecord {
                        seq: EventSeq::ZERO,
                        at: chrono::DateTime::UNIX_EPOCH,
                        event: SessionEvent::Status {
                            status: SessionStatus::Running,
                        },
                    },
                })
                .await;
            let record = phase("read real framed record", follow.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            phase(
                "queue admitted record",
                fixture
                    .selected
                    .send_guarded(&sender, scope.envelope(Payload::Records(vec![record]))),
            )
            .await
            .unwrap();
            fixture
                .peer
                .push(ServerFrame::SubscriptionEnded {
                    subscription: SubscriptionId::new(1),
                    end,
                })
                .await;
            let payload = match phase("read real end or error frame", follow.next())
                .await
                .unwrap()
            {
                None => Payload::Eof,
                Some(Err(error)) => Payload::Lost(ActFailed {
                    code: None,
                    message: error.to_string(),
                }),
                Some(Ok(_)) => panic!("unexpected extra record"),
            };
            phase(
                "queue admitted terminal frame",
                fixture
                    .selected
                    .send_guarded(&sender, scope.envelope(payload)),
            )
            .await
            .unwrap();
            fixture.state.clients["fixture"].retire();
            for _part in 0..QUEUED_PARTS {
                let envelope = phase("receive queued original payload", receiving.recv())
                    .await
                    .unwrap();
                assert_eq!(envelope.binding(), fixture.binding);
                assert_eq!(envelope.context(), &fixture.context);
                assert_eq!(envelope.operation(), None);
                assert_eq!(
                    envelope.install(|_, _| panic!("retired data reached receiver")),
                    Err(Problem::Delivery(Refusal::Changed))
                );
            }
        }
    })
    .await;
}

#[tokio::test]
async fn same_connection_reattach_rejects_old_binding_before_data_installation() {
    within(async {
        let mut fixture = fixture().await;
        let mut conflicting = fixture.binding;
        conflicting.resident.address.node = vessel_proto::NodeId::random();
        assert!(matches!(
            Scope::capture(&fixture.selected, &fixture.context, conflicting),
            Err(Refusal::Changed)
        ));
        let old = Scope::capture(&fixture.selected, &fixture.context, fixture.binding).unwrap();
        let queued = old.envelope(Payload::Metadata(Box::new(peer::info(
            fixture.binding.resident.address.session,
        ))));
        let fresh = fixture
            .residents
            .begin_follow(fixture.binding.resident)
            .unwrap();
        let mut count = 0;
        assert!(matches!(
            install(&mut fixture.residents, queued, &mut count),
            Err(Problem::Resident(_))
        ));
        assert_eq!(count, 0);
        let current = Scope::capture(&fixture.selected, &fixture.context, fresh).unwrap();
        install(
            &mut fixture.residents,
            current.envelope(Payload::Metadata(Box::new(peer::info(
                fresh.resident.address.session,
            )))),
            &mut count,
        )
        .unwrap();
        install(
            &mut fixture.residents,
            current.envelope(Payload::Records(Vec::new())),
            &mut count,
        )
        .unwrap();
        assert_eq!(count, 2);
        let wrong = current.envelope(Payload::Metadata(Box::new(peer::info(SessionId::random()))));
        assert_eq!(
            install(&mut fixture.residents, wrong, &mut count),
            Err(Problem::WrongSession)
        );
        assert_eq!(count, 2);
    })
    .await;
}

#[tokio::test]
async fn a_follow_failure_retires_only_its_binding_on_a_still_active_connection() {
    within(async {
        let mut fixture = fixture().await;
        let original =
            Scope::capture(&fixture.selected, &fixture.context, fixture.binding).unwrap();
        original
            .envelope(Payload::Lost(ActFailed {
                code: None,
                message: "follow ended".to_owned(),
            }))
            .install(|binding, payload| {
                assert!(matches!(payload, Payload::Lost(_)));
                fixture.residents.retire_follow(binding)
            })
            .unwrap();
        let fresh = fixture
            .residents
            .begin_follow(fixture.binding.resident)
            .unwrap();
        let current = Scope::capture(&fixture.selected, &fixture.context, fresh).unwrap();
        assert!(matches!(
            original
                .envelope(Payload::Eof)
                .install(|binding, _| { fixture.residents.retire_follow(binding) }),
            Err(Problem::Resident(_))
        ));
        let mut count = 0;
        install(
            &mut fixture.residents,
            current.envelope(Payload::Records(Vec::new())),
            &mut count,
        )
        .unwrap();
        assert_eq!(count, 1);
    })
    .await;
}

#[tokio::test]
async fn canonical_operation_kind_and_single_use_ticket_precede_completion() {
    within(async {
        let mut fixture = fixture().await;
        let scope = Scope::capture(&fixture.selected, &fixture.context, fixture.binding).unwrap();
        let operation = Operation {
            kind: OperationKind::Lifecycle,
            ticket: fixture
                .residents
                .admit_operation(fixture.binding, OperationKind::Lifecycle)
                .unwrap(),
        };
        let mut count = 0;
        let completion = || {
            scope.envelope(Payload::Completion {
                operation,
                result: Box::new(Ok(Acted::Done)),
            })
        };
        assert_eq!(completion().operation(), Some(operation));
        install(&mut fixture.residents, completion(), &mut count).unwrap();
        assert!(matches!(
            install(&mut fixture.residents, completion(), &mut count),
            Err(Problem::Resident(_))
        ));
        assert_eq!(count, 1);
        let wrong = scope.envelope(Payload::Completion {
            operation: Operation {
                kind: OperationKind::Composer,
                ..operation
            },
            result: Box::new(Ok(Acted::Done)),
        });
        assert!(matches!(
            install(&mut fixture.residents, wrong, &mut count),
            Err(Problem::Resident(_))
        ));
        assert_eq!(count, 1);
    })
    .await;
}

#[tokio::test]
async fn guardless_refusals_downgrade_only_the_original_pending_admission() {
    within(async {
        let mut fixture = fixture().await;
        let operation = Operation {
            kind: OperationKind::Lifecycle,
            ticket: fixture
                .residents
                .admit_operation(fixture.binding, OperationKind::Lifecycle)
                .unwrap(),
        };
        CorrelatedRefusal::new(
            fixture.context.clone(),
            fixture.binding,
            Some(operation),
            FailurePhase::NotSent,
            Refusal::Changed,
        )
        .apply(|context, binding, admission, failure_phase, reason| {
            assert_eq!(context, &fixture.context);
            assert_eq!(failure_phase, FailurePhase::NotSent);
            assert_eq!(reason, Refusal::Changed);
            let admission = admission.unwrap();
            fixture
                .residents
                .settle_operation(binding, admission.kind, admission.ticket)
        })
        .unwrap();
        let old = fixture.binding;
        let fresh = fixture.residents.begin_follow(old.resident).unwrap();
        let successor = fixture
            .residents
            .admit_operation(fresh, OperationKind::Lifecycle)
            .unwrap();
        let refusal = CorrelatedRefusal::new(
            fixture.context.clone(),
            old,
            Some(operation),
            FailurePhase::Uncertain,
            Refusal::Closed,
        );
        assert!(
            refusal
                .apply(|_, binding, _, _, _| {
                    fixture.residents.validate_binding(binding)?;
                    let _retired = fixture.residents.retire_current(binding.resident)?;
                    Ok(())
                })
                .is_err()
        );
        assert!(fixture.residents.validate_binding(fresh).is_ok());
        fixture
            .residents
            .settle_operation(fresh, OperationKind::Lifecycle, successor)
            .unwrap();
    })
    .await;
}
