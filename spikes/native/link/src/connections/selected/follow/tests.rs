//! Actual framed attach and buffered delivery; retirement never retargets.
use super::*;
use crate::connections::Connected;
use crate::testing::{Home, peer, state};
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
    let (_home, state, mut peer, selected) = setup(true).await;
    let session = SessionId::random();
    let attaching = selected.follow_session(session, EventSeq::ZERO);
    tokio::pin!(attaching);
    let request = tokio::select! {
        result = &mut attaching => panic!("attach completed without peer reply: {}", result.is_ok()),
        request = peer.requests.as_mut().unwrap().recv() => request.unwrap(),
    };
    assert!(matches!(request.call, Call::Subscribe(_)));
    state.clients["fixture"].retire();
    let _reply = request
        .answer
        .send(Outcome::Ok(Reply::Subscribe(Subscribed {
            subscription: SubscriptionId::new(1),
            info: peer::info(session),
        })));
    assert!(matches!(
        attaching.await,
        Err(RequestFailure::Refused(Refusal::Changed))
    ));
    assert!(matches!(
        selected.follow_session(session, EventSeq::ZERO).await,
        Err(RequestFailure::Refused(Refusal::Changed))
    ));
    assert_eq!(peer.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn buffered_record_and_metadata_are_refused_after_retirement() {
    let (_home, state, peer, selected) = setup(false).await;
    let mut follow = selected
        .follow_session(SessionId::random(), EventSeq::ZERO)
        .await
        .unwrap();
    assert_eq!(follow.context(), &selected.context);
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
    let sessions = selected.call(ListSessions).await.unwrap();
    assert!(sessions.is_empty());
    state.clients["fixture"].retire();
    assert_eq!(follow.info().unwrap_err(), Refusal::Changed);
    assert_eq!(follow.next().await.unwrap_err(), Refusal::Changed);
    assert_eq!(selected.retirement().await, Refusal::Changed);
}

#[tokio::test]
async fn completion_fence_rejects_success_error_and_eof_even_if_the_future_is_ready() {
    for result in [
        Ok(Some(7)),
        Ok(None),
        Err(vessel_client::CallError::Disconnected),
    ] {
        let (_home, state, _peer, selected) = setup(false).await;
        let result = selected
            .guard(async {
                state.clients["fixture"].retire();
                result
            })
            .await;
        assert!(matches!(
            result,
            Err(RequestFailure::Refused(Refusal::Changed))
        ));
    }
}

#[tokio::test]
async fn preflight_never_searches_another_alias_or_accepts_a_different_actual_node() {
    let (_home, mut state, _peer, selected) = setup(false).await;
    let connected = state.clients.remove("fixture").unwrap();
    let _other = state.clients.insert("other".into(), connected.clone());
    assert!(matches!(
        state.selected_connection(&selected.context),
        Err(Refusal::Changed)
    ));
    let mut replacement = peer::connect("different-node").await;
    let wrong = Connected::new(
        replacement.client.take().unwrap(),
        Vec::new(),
        connected.binding.clone(),
        selected.context.clone(),
    );
    let _other = state.clients.insert("fixture".into(), wrong);
    assert!(matches!(
        state.selected_connection(&selected.context),
        Err(Refusal::Changed)
    ));
    assert!(replacement.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn closed_reports_return_to_the_actor_and_never_retire_a_new_admission() {
    let (_home, mut state, mut peer, mut selected) = setup(false).await;
    let (commands, mut receiving) = tokio::sync::mpsc::channel(crate::CHANNEL);
    selected.commands = commands.downgrade();
    peer.task.take().unwrap().abort();
    tokio::time::timeout(crate::testing::DEADLINE, selected.client.closed())
        .await
        .unwrap();
    assert!(matches!(
        selected.call(ListSessions).await,
        Err(RequestFailure::Refused(Refusal::Closed))
    ));
    let crate::Command::RetireConnection(tag) = receiving.try_recv().unwrap() else {
        panic!("closure must be ordered by the actor");
    };
    assert_eq!(tag, selected.context.tag());
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
    state.closed_connection(&tag).await;
    assert!(state.clients["fixture"].matches(&context));
}
