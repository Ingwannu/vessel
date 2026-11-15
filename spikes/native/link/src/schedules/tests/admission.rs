//! Feature-level proof: an old schedule intent never moves to a replacement
//! admission, even if its alias or authenticated `NodeId` is reused.
use super::*;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::connection::{ConnectionContext, Refusal};
use vessel_client::book::BookEntry;

pub(super) fn insert(
    state: &mut crate::State,
    name: &str,
    peer: &mut peer::Peer,
) -> ConnectionContext {
    let client = peer.client.take().unwrap();
    let binding = BookEntry {
        address: vessel_client::NodeAddress::Ws("ws://127.0.0.1:0/rpc".parse().unwrap()),
        token: None,
        credential: None,
        id: Some(client.node().id),
    };
    let connected = state
        .admit_connection(name, &binding, client, Vec::new())
        .unwrap();
    let context = connected.context.clone();
    if let Some(old) = state.clients.insert(name.to_owned(), connected) {
        old.retire();
    }
    context
}
fn request(context: ConnectionContext) -> Request {
    Request::on(
        context,
        Tickets::default().issue().unwrap(),
        Ask::Run("00000000-0000-4000-8000-000000000001".parse().unwrap()),
    )
}
async fn answer(events: &mut tokio::sync::mpsc::Receiver<crate::Event>) -> Answer {
    let event = phase("schedule answer", events.recv()).await.unwrap();
    let crate::Event::Schedules(Message::Replied(answer)) = event else {
        panic!("wrong answer")
    };
    answer
}
#[tokio::test]
async fn a_queued_write_is_not_retargeted_to_a_replacement_or_another_alias() {
    within(async {
        for replacement_alias in ["laptop", "other"] {
            let home = Home::new();
            let (mut state, mut events) = state(&home);
            let mut original = phase("original peer", peer::connect("laptop")).await;
            let context = insert(&mut state, "laptop", &mut original);
            let request = request(context.clone());
            let mut replacement = phase(
                "replacement peer",
                peer::connect_with_node(replacement_alias, context.node()),
            )
            .await;
            let original = state.clients.remove("laptop").unwrap();
            original.retire();
            let fresh = insert(&mut state, replacement_alias, &mut replacement);
            assert_ne!(context.epoch(), fresh.epoch());
            phase("schedule dispatch", state.schedule(request.clone())).await;
            let answer = answer(&mut events).await;
            assert_eq!(answer.request, request);
            assert!(matches!(
                answer.result,
                Err(ScheduleError::NotSelected(Refusal::Changed))
            ));
            assert!(replacement.calls.lock().unwrap().is_empty());
        }
    })
    .await;
}
#[tokio::test]
async fn retirement_before_first_poll_refuses_the_original_call_without_replay() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase("original peer", peer::connect("laptop")).await;
        let context = insert(&mut state, "laptop", &mut peer);
        let selected = state.selected_connection(&context).unwrap();
        state.clients["laptop"].retire();
        let request = request(context);
        phase(
            "hub admission",
            Hub::default().ask(Ok(selected), request.clone(), state.events.clone()),
        )
        .await;
        let answer = answer(&mut events).await;
        assert_eq!(answer.request, request);
        assert!(matches!(
            answer.result,
            Err(ScheduleError::Retired(Refusal::Changed))
        ));
        assert!(peer.calls.lock().unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn a_sent_write_retiring_during_await_is_uncertain_and_keeps_its_original_context() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut original = phase("controlled original", peer::controlled("laptop")).await;
        let context = insert(&mut state, "laptop", &mut original);
        let request = request(context.clone());
        phase("schedule dispatch", state.schedule(request.clone())).await;
        let pending = phase(
            "actual run request frame",
            original.requests.as_mut().unwrap().recv(),
        )
        .await
        .unwrap();
        assert!(matches!(pending.call, vessel_proto::Call::RunSchedule(_)));
        state.clients["laptop"].retire();
        let mut replacement = phase(
            "same-node new transport",
            peer::connect_with_node("laptop", context.node()),
        )
        .await;
        let fresh = insert(&mut state, "laptop", &mut replacement);
        assert_ne!(fresh.epoch(), context.epoch());
        let answer = answer(&mut events).await;
        assert_eq!(answer.request, request);
        let error = answer.result.unwrap_err();
        assert!(matches!(error, ScheduleError::Retired(Refusal::Changed)));
        assert_eq!(error.failure(request.ask), Failure::Unconfirmed);
        // The original node may still finish after the local waiter retires.
        // No reply from it may be replayed through the new admitted client.
        let _closed = pending
            .answer
            .send(vessel_proto::Outcome::Err(WireError::new(
                ErrorCode::Unsupported,
                "synthetic late completion",
            )));
        assert!(replacement.calls.lock().unwrap().is_empty());
    })
    .await;
}
