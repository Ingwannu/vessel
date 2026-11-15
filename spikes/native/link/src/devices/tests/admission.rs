//! Real framed `NodeClient` requests, with explicit peer barriers rather than
//! sleeps. These prove adapter admission, not the node's authorization policy.
use crate::connections::{Connected, Validity};
use crate::devices::{Ask, Outcome};
use crate::testing::{Home, peer, phase, state, within};
use crate::{Event, State};
use tokio::sync::mpsc;
use vessel_app_view::devices::{ConnectionContext, MachineKey, Problem, Tickets};
use vessel_proto::{Ack, Call, ErrorCode, Reply};

const BOOK: &str = "[nodes.fixture]\nurl = \"ws://127.0.0.1:0/rpc\"\n";

async fn install(state: &mut State, peer: &mut peer::Peer) -> ConnectionContext {
    let client = peer.client.take().unwrap();
    let context = state
        .admissions
        .admit(MachineKey::from_book("fixture"), client.node().id)
        .unwrap();
    let entry = phase("read the isolated node book", state.saved_entries())
        .await
        .unwrap()
        .remove(0)
        .1;
    if let Some(old) = state.clients.insert(
        "fixture".into(),
        Connected::new(client, Vec::new(), entry, context.clone()),
    ) {
        old.retire();
    }
    context
}
async fn answer(events: &mut mpsc::Receiver<Event>) -> crate::devices::Answer {
    phase("receive the correlated device answer", async {
        loop {
            if let Event::Devices(answer) = events.recv().await.unwrap() {
                return answer;
            }
        }
    })
    .await
}
async fn request(requests: &mut mpsc::Receiver<peer::Request>) -> peer::Request {
    phase("receive the expected framed request", requests.recv())
        .await
        .expect("the framed peer did not close before the request")
}
fn home() -> Home {
    let home = Home::new();
    std::fs::write(home.book(), BOOK).unwrap();
    home
}

#[tokio::test]
async fn stale_admission_never_sends_a_request_to_the_replacement_under_the_alias() {
    within(async {

    let home = home();
    let (mut state, mut events) = state(&home);
    let mut first = peer::controlled("first").await;
    let old = install(&mut state, &mut first).await;
    let mut next = peer::controlled("next").await;
    let current = install(&mut state, &mut next).await;
    let ticket = Tickets::default().next_ticket().unwrap();
    phase("dispatch the stale-context list", state.devices(&old, ticket, Ask::List)).await;
    let response = answer(&mut events).await;
    assert_eq!(response.context, old);
    assert!(
        matches!(response.outcome, Outcome::Listed(t, Err(Problem::TargetChanged)) if t == ticket)
    );
    assert!(first.calls.lock().unwrap().is_empty());
    assert!(next.calls.lock().unwrap().is_empty());
    assert!(state.clients["fixture"].matches(&current));

    })
    .await;
}

#[tokio::test]
async fn retiring_a_queued_call_before_its_first_poll_refuses_without_rpc() {
    within(async {

    let home = home();
    let (mut state, mut events) = state(&home);
    let mut peer = peer::controlled("fixture").await;
    let context = install(&mut state, &mut peer).await;
    let ticket = Tickets::default().next_ticket().unwrap();
    phase("admit the queued revoke without polling its worker", state.devices(&context, ticket, Ask::Revoke { device: vessel_proto::DeviceId::random() })).await;
    // current-thread runtime: the spawned call has not been polled yet.
    state.clients["fixture"].retire();
    let response = answer(&mut events).await;
    assert_eq!(response.context, context);
    assert!(
        matches!(response.outcome, Outcome::Revoked(t, Err(Problem::TargetChanged)) if t == ticket)
    );
    assert!(peer.calls.lock().unwrap().is_empty());

    })
    .await;
}

#[tokio::test]
async fn replacement_after_rpc_admission_never_replays_the_write_on_the_new_client() {
    within(async {
        let home = home();
        let (mut state, mut events) = state(&home);
        let mut old_peer = peer::controlled("first").await;
        let old = install(&mut state, &mut old_peer).await;
        let ticket = Tickets::default().next_ticket().unwrap();
        let device = vessel_proto::DeviceId::random();
        phase(
            "admit the original revoke request",
            state.devices(&old, ticket, Ask::Revoke { device }),
        )
        .await;
        let admitted = request(old_peer.requests.as_mut().unwrap()).await;
        assert!(matches!(admitted.call, Call::RevokeDevice(ask) if ask.device == device));
        let mut next_peer = peer::controlled("next").await;
        let next = install(&mut state, &mut next_peer).await;
        let _late_reply = admitted
            .answer
            .send(vessel_proto::Outcome::Ok(Reply::RevokeDevice(Ack {})));
        let response = answer(&mut events).await;
        assert_eq!(response.context, old);
        assert!(matches!(response.outcome, Outcome::Revoked(t, _) if t == ticket));
        assert_eq!(old_peer.calls.lock().unwrap().len(), 1);
        assert!(next_peer.calls.lock().unwrap().is_empty());
        assert!(state.clients["fixture"].matches(&next));
    })
    .await;
}

async fn reply_to_denied_sessions(requests: &mut mpsc::Receiver<peer::Request>) {
    let sessions = request(requests).await;
    assert!(matches!(sessions.call, Call::ListSessions(_)));
    sessions
        .answer
        .send(vessel_proto::Outcome::Err(vessel_proto::WireError::new(
            ErrorCode::Forbidden,
            "session read refused",
        )))
        .unwrap();
    let devices = request(requests).await;
    assert!(matches!(devices.call, Call::ListDevices(_)));
    devices
        .answer
        .send(vessel_proto::Outcome::Ok(Reply::ListDevices(Vec::new())))
        .unwrap();
}

#[tokio::test]
async fn forbidden_session_data_keeps_authenticated_context_and_usable_device_rpc() {
    within(async {
        let home = home();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::controlled("fixture").await;
        let context = install(&mut state, &mut peer).await;
        let mut requests = peer.requests.take().unwrap();
        let exercising = async {
            phase("refresh facts after the semantic refusal", state.refresh()).await;
            assert!(state.clients["fixture"].matches(&context));
            assert_eq!(
                *state.clients["fixture"].validity().borrow(),
                Validity::Active
            );
            let facts = std::iter::from_fn(|| events.try_recv().ok())
                .filter_map(|event| {
                    if let Event::Fleet(fleet) = event {
                        Some(
                            fleet
                                .machines
                                .into_iter()
                                .filter_map(|machine| machine.connection_context())
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert!(!facts.is_empty());
            assert!(facts.iter().all(|facts| facts == &vec![context.clone()]));
            let ticket = Tickets::default().next_ticket().unwrap();
            phase(
                "dispatch devices over the retained connection",
                state.devices(&context, ticket, Ask::List),
            )
            .await;
            assert!(
                matches!(answer(&mut events).await.outcome, Outcome::Listed(t, Ok(devices)) if t == ticket && devices.is_empty())
            );
        };
        // A scoped responder is dropped with the exercising future on any failed
        // phase; no detached responder task outlives a failed assertion.
        let ((), ()) = tokio::join!(
            phase("exercise the session-refused admission", exercising),
            phase(
                "serve the session-refused peer",
                reply_to_denied_sessions(&mut requests),
            ),
        );
    })
    .await;
}
