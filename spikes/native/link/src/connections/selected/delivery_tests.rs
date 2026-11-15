//! Real framed clients plus bounded queues; no node, agent or reconnect is started.
use super::*;
use crate::connections::Connected;
use crate::testing::{Home, peer, phase, state, within};
use std::sync::Arc;
use vessel_app_view::connection::MachineKey;
use vessel_proto::{ListSessions, Outcome, Reply};
const VALIDITY_PROBES: usize = 64;
const QUEUE_CAPACITY: usize = 1;

struct Fixture {
    _home: Home,
    state: State,
    peer: peer::Peer,
    selected: SelectedConnection,
    context: ConnectionContext,
}
async fn fixture(controlled: bool) -> Fixture {
    let home = Home::new();
    let (mut state, _events) = state(&home);
    let mut peer = if controlled {
        peer::controlled("guard-fixture").await
    } else {
        peer::connect("guard-fixture").await
    };
    let client = peer.client.take().unwrap();
    let context = state
        .admissions
        .admit(MachineKey::from_book("guard-fixture"), client.node().id)
        .unwrap();
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    };
    let _previous = state.clients.insert(
        "guard-fixture".to_owned(),
        Connected::new(client, Vec::new(), binding, context.clone()),
    );
    let selected = state.selected_connection(&context).unwrap();
    Fixture {
        _home: home,
        state,
        peer,
        selected,
        context,
    }
}

#[tokio::test]
async fn active_unary_guard_survives_task_completion_but_does_not_retain_transport() {
    within(async {
        let Fixture {
            mut state,
            mut peer,
            selected,
            context,
            _home,
        } = fixture(false).await;
        let guard = selected.delivery_guard();
        let selected_again = state.selected_connection(&context).unwrap();
        assert!(Arc::ptr_eq(&selected.lease, &selected_again.lease));
        let before = Arc::strong_count(&selected.lease);
        let clone = guard.clone();
        assert_eq!(Arc::strong_count(&selected.lease), before);
        assert!(
            phase("original unary", selected.call(ListSessions))
                .await
                .unwrap()
                .is_empty()
        );
        drop(selected_again);
        drop(selected);
        assert_eq!(guard.validate(&context), Ok(()));
        let connection = state.clients.remove("guard-fixture").unwrap();
        connection.retire();
        drop(connection);
        phase(
            "weak queued guards allow physical transport drop",
            peer.task.take().unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(guard.validate(&context), Err(Refusal::Changed));
        assert_eq!(clone.validate(&context), Err(Refusal::Changed));
        assert_eq!(format!("{guard:?}"), "DeliveryGuard(<opaque>)");
    })
    .await;
}

#[tokio::test]
async fn dropped_validity_owner_refuses_last_active_value_while_old_work_retains_client() {
    within(async {
        let Fixture {
            mut state,
            mut peer,
            selected,
            context,
            _home,
        } = fixture(false).await;
        let guard = selected.delivery_guard();
        drop(state.clients.remove("guard-fixture").unwrap());
        assert!(!peer.task.as_ref().unwrap().is_finished());
        assert_eq!(guard.validate(&context), Err(Refusal::Changed));
        assert!(matches!(
            selected.call(ListSessions).await,
            Err(RequestFailure::Refused(Refusal::Changed))
        ));
        assert!(peer.calls.lock().unwrap().is_empty());
        drop(selected);
        phase(
            "transport drops after final in-flight owner",
            peer.task.take().unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(guard.validate(&context), Err(Refusal::Changed));
    })
    .await;
}

#[tokio::test]
async fn original_closed_transport_and_wrong_epoch_refuse_without_wire_requests() {
    within(async {
        let Fixture {
            mut state,
            mut peer,
            selected,
            context,
            _home,
        } = fixture(false).await;
        let guard = selected.delivery_guard();
        let successor = state
            .admissions
            .admit(MachineKey::from_book("guard-fixture"), context.node())
            .unwrap();
        assert_eq!(guard.validate(&successor), Err(Refusal::Changed));
        let other_alias = state
            .admissions
            .admit(MachineKey::from_book("another-alias"), context.node())
            .unwrap();
        assert_eq!(guard.validate(&other_alias), Err(Refusal::Changed));
        let task = peer.task.take().unwrap();
        task.abort();
        assert!(phase("close framed peer", task).await.is_err());
        phase(
            "observe original physical close",
            selected.lease.client.closed(),
        )
        .await;
        let strong = Arc::strong_count(&selected.lease);
        for _ in 0..VALIDITY_PROBES {
            assert_eq!(guard.validate(&context), Err(Refusal::Closed));
        }
        assert_eq!(Arc::strong_count(&selected.lease), strong);
        assert!(peer.calls.lock().unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn retired_inflight_unary_keeps_its_original_transport_only_until_work_ends() {
    within(async {
        let Fixture { mut state, mut peer, selected, context, _home } = fixture(true).await;
        let guard = selected.delivery_guard();
        {
            let calling = selected.call(ListSessions);
            tokio::pin!(calling);
            let request = phase("observe admitted unary", async {
                tokio::select! {
                    result = &mut calling => panic!("unary unexpectedly completed: {}", result.is_ok()),
                    request = peer.requests.as_mut().unwrap().recv() => request.unwrap(),
                }
            }).await;
            let connection = state.clients.remove("guard-fixture").unwrap();
            connection.retire();
            drop(connection);
            assert!(!peer.task.as_ref().unwrap().is_finished());
            let _sent = request.answer.send(Outcome::Ok(Reply::ListSessions(Vec::new())));
            assert!(matches!(phase("retired unary result", calling).await, Err(RequestFailure::Refused(Refusal::Changed))));
            assert_eq!(guard.validate(&context), Err(Refusal::Changed));
        }
        drop(selected);
        phase("retired work releases transport despite retained guard", peer.task.take().unwrap()).await.unwrap();
        assert_eq!(guard.validate(&context), Err(Refusal::Changed));
    }).await;
}

#[tokio::test]
async fn queued_item_error_eof_and_unary_envelopes_recheck_after_send_before_install() {
    within(async {
        // Symbolic envelopes exercise queue validity, not real stream EOF/error frames.
        for payload in [Ok(Some(1_u8)), Err(2_u8), Ok(None), Ok(Some(3_u8))] {
            let Fixture {
                state,
                selected,
                context,
                _home,
                peer: _peer,
            } = fixture(false).await;
            let (send, mut receive) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
            let guard = selected.delivery_guard();
            selected
                .send_guarded(&send, (guard, payload))
                .await
                .unwrap();
            let (guard, queued) = phase("take accepted queued envelope", receive.recv())
                .await
                .unwrap();
            assert_eq!(queued, payload);
            assert_eq!(guard.validate(&context), Ok(()));
            state.clients["guard-fixture"].retire();
            assert_eq!(guard.validate(&context), Err(Refusal::Changed));
        }
    })
    .await;
}

#[tokio::test]
async fn bounded_send_is_cancelled_by_retirement_and_queued_guard_cannot_install() {
    within(async {
        let Fixture {
            state,
            selected,
            context,
            _home,
            peer: _peer,
        } = fixture(false).await;
        let (send, mut receive) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
        selected
            .send_guarded(&send, selected.delivery_guard())
            .await
            .unwrap();
        let sending = selected.send_guarded(&send, selected.delivery_guard());
        tokio::pin!(sending);
        let pending = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(sending.as_mut().poll(cx).is_pending())
        })
        .await;
        assert!(pending);
        state.clients["guard-fixture"].retire();
        assert_eq!(
            phase("cancel full-channel send", sending).await,
            Err(queue::SendFailure::Refused(Refusal::Changed))
        );
        let guard = phase("drain already queued envelope", receive.recv())
            .await
            .unwrap();
        assert_eq!(guard.validate(&context), Err(Refusal::Changed));
        assert!(receive.try_recv().is_err());
    })
    .await;
}

#[tokio::test]
async fn closed_app_receiver_does_not_retire_the_original_connection() {
    within(async {
        let Fixture {
            selected,
            context,
            state: _state,
            _home,
            peer: _peer,
        } = fixture(false).await;
        let guard = selected.delivery_guard();
        let (send, receive) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
        drop(receive);
        assert_eq!(
            selected.send_guarded(&send, guard.clone()).await,
            Err(queue::SendFailure::ReceiverClosed)
        );
        assert_eq!(guard.validate(&context), Ok(()));
    })
    .await;
}

#[tokio::test]
async fn closed_identity_still_selects_and_reports_closed_retirement_once() {
    within(async {
        let Fixture {
            mut state,
            mut peer,
            selected,
            context,
            _home,
        } = fixture(false).await;
        let (commands, mut receiver) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
        state.commands = commands.downgrade();
        let task = peer.task.take().unwrap();
        task.abort();
        assert!(phase("stop framed transport", task).await.is_err());
        phase(
            "observe selected transport closed",
            selected.lease.client.closed(),
        )
        .await;
        let admitted = state.selected_connection(&context).unwrap();
        assert!(matches!(
            admitted.call(ListSessions).await,
            Err(RequestFailure::Refused(Refusal::Closed))
        ));
        let command = phase("receive exactly one closed report", receiver.recv())
            .await
            .unwrap();
        assert!(matches!(command, Command::RetireConnection(tag) if tag == context.tag()));
        assert!(receiver.try_recv().is_err());
        assert!(peer.calls.lock().unwrap().is_empty());
    })
    .await;
}
