//! Actual framed attach and buffered delivery; retirement never retargets.
use super::*;
use crate::connections::Connected;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::connection::MachineKey;
use vessel_proto::{
    Call, ListSessions, Outcome, Reply, ServerFrame, SessionEvent, SessionStatus, Subscribed,
    SubscriptionId,
};

async fn setup(controlled: bool) -> (Home, crate::State, peer::Peer, SelectedConnection) {
    let home = Home::new();
    let (mut state, _events) = state(&home);
    let mut peer = if controlled {
        peer::controlled("fixture").await
    } else {
        peer::connect("fixture").await
    };
    let client = peer.client.take().unwrap();
    let context = state
        .admissions
        .admit(MachineKey::from_book("fixture"), client.node().id)
        .unwrap();
    let entry = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    };
    let _old = state.clients.insert(
        "fixture".into(),
        Connected::new(client, Vec::new(), entry, context.clone()),
    );
    let selected = state.selected_connection(&context).unwrap();
    (home, state, peer, selected)
}

#[tokio::test]
async fn retired_attach_is_not_dispatched_and_an_admitted_attach_is_not_exposed() {
    within(async {
        let (_home, state, mut peer, selected) = setup(true).await;
        let session = SessionId::random();
        let attaching = selected.follow_session(session, EventSeq::ZERO);
        tokio::pin!(attaching);
        let request = phase("observe the initial subscribe request", async {
            tokio::select! {
                result = &mut attaching => {
                    panic!("attach completed without peer reply: {}", result.is_ok());
                }
                request = peer.requests.as_mut().unwrap().recv() => request.unwrap(),
            }
        })
        .await;
        assert!(matches!(request.call, Call::Subscribe(_)));
        state.clients["fixture"].retire();
        let _reply = request
            .answer
            .send(Outcome::Ok(Reply::Subscribe(Subscribed {
                subscription: SubscriptionId::new(1),
                info: peer::info(session),
            })));
        assert!(matches!(
            phase("refuse the retired attach reply", attaching).await,
            Err(RequestFailure::Refused(Refusal::Changed))
        ));
        assert!(matches!(
            phase(
                "refuse attaching after retirement",
                selected.follow_session(session, EventSeq::ZERO)
            )
            .await,
            Err(RequestFailure::Refused(Refusal::Changed))
        ));
        assert_eq!(peer.calls.lock().unwrap().len(), 1);
    })
    .await;
}

#[tokio::test]
async fn buffered_record_and_metadata_are_refused_after_retirement() {
    within(async {
        let (_home, state, peer, selected) = setup(false).await;
        let mut follow = phase(
            "attach the buffered subscription",
            selected.follow_session(SessionId::random(), EventSeq::ZERO),
        )
        .await
        .unwrap();
        assert_eq!(follow.context(), &selected.lease.context);
        assert!(follow.info().is_ok());
        peer.push(ServerFrame::Event {
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
        // The response follows the event on the same framed reader: the item is
        // actually buffered before retirement, not merely scheduled for sending.
        let sessions = phase(
            "observe injected event before barrier reply",
            selected.call(ListSessions),
        )
        .await
        .unwrap();
        assert!(sessions.is_empty());
        state.clients["fixture"].retire();
        assert_eq!(follow.info().unwrap_err(), Refusal::Changed);
        assert_eq!(
            phase("refuse buffered data after retirement", follow.next())
                .await
                .unwrap_err(),
            Refusal::Changed
        );
        assert_eq!(
            phase("observe the selected retirement", selected.retirement()).await,
            Refusal::Changed
        );
    })
    .await;
}

#[tokio::test]
async fn completion_fence_rejects_success_error_and_eof_even_if_the_future_is_ready() {
    within(async {
        for result in [
            Ok(Some(7)),
            Ok(None),
            Err(vessel_client::CallError::Disconnected),
        ] {
            let (_home, state, _peer, selected) = setup(false).await;
            let result = phase(
                "fence a completed original-client future",
                selected.guard(async {
                    state.clients["fixture"].retire();
                    result
                }),
            )
            .await;
            assert!(matches!(
                result,
                Err(RequestFailure::Refused(Refusal::Changed))
            ));
        }
    })
    .await;
}

#[tokio::test]
async fn preflight_never_searches_another_alias_or_accepts_a_different_actual_node() {
    within(async {
        let (_home, mut state, _peer, selected) = setup(false).await;
        let connected = state.clients.remove("fixture").unwrap();
        let _other = state.clients.insert("other".into(), connected.clone());
        assert!(matches!(
            state.selected_connection(&selected.lease.context),
            Err(Refusal::Changed)
        ));
        let mut replacement = peer::connect("different-node").await;
        let wrong = Connected::new(
            replacement.client.take().unwrap(),
            Vec::new(),
            connected.binding.clone(),
            selected.lease.context.clone(),
        );
        let _other = state.clients.insert("fixture".into(), wrong);
        assert!(matches!(
            state.selected_connection(&selected.lease.context),
            Err(Refusal::Changed)
        ));
        assert!(replacement.calls.lock().unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn closed_reports_return_to_the_actor_and_never_retire_a_new_admission() {
    within(async {
        let (_home, mut state, mut peer, mut selected) = setup(false).await;
        let (commands, mut receiving) = tokio::sync::mpsc::channel(crate::CHANNEL);
        selected.commands = commands.downgrade();
        peer.task.take().unwrap().abort();
        phase(
            "observe original transport closure",
            selected.lease.client.closed(),
        )
        .await;
        assert!(matches!(
            phase(
                "refuse a call on the closed transport",
                selected.call(ListSessions)
            )
            .await,
            Err(RequestFailure::Refused(Refusal::Closed))
        ));
        let crate::Command::RetireConnection(tag) = receiving.try_recv().unwrap() else {
            panic!("closure must be ordered by the actor");
        };
        assert_eq!(tag, selected.lease.context.tag());
        let mut replacement = peer::connect("new-admission").await;
        let context = state
            .admissions
            .admit(
                MachineKey::from_book("fixture"),
                replacement.client.as_ref().unwrap().node().id,
            )
            .unwrap();
        let binding = state.clients["fixture"].binding.clone();
        let _old = state.clients.insert(
            "fixture".into(),
            Connected::new(
                replacement.client.take().unwrap(),
                Vec::new(),
                binding,
                context.clone(),
            ),
        );
        phase(
            "apply the obsolete closure report",
            state.closed_connection(&tag),
        )
        .await;
        assert!(state.clients["fixture"].matches(&context));
    })
    .await;
}
