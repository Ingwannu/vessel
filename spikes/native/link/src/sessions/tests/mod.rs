//! Real typed RPC tests of session resolution and lifecycle acknowledgement.
mod bound;
mod peer;
use crate::{Connected, Event, State};
use std::time::Duration;
use tokio::sync::mpsc;
use vessel_app_view::machine_book::Name;
use vessel_app_view::sessions::{
    Lifecycle, LifecycleAction, LifecycleTarget, NavigationTarget, NavigationTicket, SessionAddress,
};
use vessel_proto::{Call, NodeId, SessionId};
const TEST_WAIT: Duration = super::navigation::LOOKUP_DEADLINE.saturating_mul(2);
fn connected(state: &mut State, host: &str, client: vessel_client::NodeClient) -> Connected {
    state
        .admit_connection(
            host,
            &vessel_client::book::BookEntry {
                address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
                token: None,
                credential: None,
                id: Some(client.node().id),
            },
            client,
            Vec::new(),
        )
        .unwrap()
}
fn state(client: vessel_client::NodeClient) -> (State, mpsc::Receiver<Event>) {
    let home = crate::testing::Home::new();
    let (mut state, receiving) = crate::testing::state(&home);
    state.book_path = crate::machines::BookPath::NoConfiguration;
    let connection = connected(&mut state, "renamed", client);
    assert!(
        state
            .clients
            .insert("renamed".to_owned(), connection)
            .is_none()
    );
    (state, receiving)
}
async fn next(events: &mut mpsc::Receiver<Event>) -> Event {
    tokio::time::timeout(TEST_WAIT, events.recv())
        .await
        .unwrap()
        .unwrap()
}
#[tokio::test]
async fn address_resolution_survives_alias_rename_and_only_reads_metadata() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let node = peer.client.node().id;
    let (mut state, mut events) = state(peer.client.clone());
    let session = SessionId::random();
    let ticket = NavigationTicket::FIRST.next().unwrap();
    state.session_navigation(super::Ask {
        ticket,
        target: NavigationTarget::Address(SessionAddress { node, session }),
    });
    let Event::Sessions(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert_eq!(answer.ticket, ticket);
    let located = answer.result.unwrap();
    assert_eq!(located.address, SessionAddress { node, session });
    assert_eq!(located.host.as_str(), "renamed");
    assert!(
        matches!(peer.calls.recv().await, Some(Call::GetSession(request)) if request.session == session)
    );
    assert!(
        peer.calls.try_recv().is_err(),
        "navigation must not resume/create an agent"
    );
}
#[tokio::test(start_paused = true)]
async fn another_node_identity_never_falls_back_to_a_similar_saved_name() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let (mut state, mut events) = state(peer.client.clone());
    state.session_navigation(super::Ask {
        ticket: NavigationTicket::FIRST,
        target: NavigationTarget::Address(SessionAddress {
            node: NodeId::random(),
            session: SessionId::random(),
        }),
    });
    let Event::Sessions(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert!(matches!(answer.result, Err(super::Failure::TimedOut)));
    assert!(peer.calls.try_recv().is_err());
}
#[tokio::test]
async fn lifecycle_archive_carries_exact_request_identity_and_authoritative_result() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let (state, mut events) = state(peer.client.clone());
    let session = SessionId::random();
    let mut lifecycle = Lifecycle::default();
    let request = lifecycle
        .begin(
            LifecycleTarget {
                host: Name::parse("renamed").unwrap(),
                session,
                node: None,
            },
            LifecycleAction::Exit,
        )
        .unwrap();
    let ticket = request.ticket;
    state.session_lifecycle(&request);
    let Event::SessionLifecycle(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert_eq!(answer.ticket, ticket);
    assert!(
        matches!(answer.result, Ok(crate::Acted::Session(info)) if info.id == session && info.marks.archived_at.is_some())
    );
    assert!(
        matches!(peer.calls.recv().await, Some(Call::MarkSession(request)) if request.session == session && request.mark == vessel_proto::Mark::Archive)
    );
    assert!(peer.calls.try_recv().is_err());
}
#[tokio::test]
async fn lifecycle_refusal_retains_typed_failure_instead_of_success() {
    let peer = peer::connect(peer::Behavior::Refuse).await;
    let (state, mut events) = state(peer.client.clone());
    let mut lifecycle = Lifecycle::default();
    let request = lifecycle
        .begin(
            LifecycleTarget {
                host: Name::parse("renamed").unwrap(),
                session: SessionId::random(),
                node: None,
            },
            LifecycleAction::Archive,
        )
        .unwrap();
    state.session_lifecycle(&request);
    let Event::SessionLifecycle(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert!(
        matches!(answer.result, Err(super::lifecycle::Failure::Refused(failed)) if failed.code == Some(vessel_proto::ErrorCode::Conflict))
    );
}

#[tokio::test]
async fn peer_cannot_redirect_an_exact_deep_link_to_another_session() {
    let peer = peer::connect(peer::Behavior::WrongSession).await;
    let node = peer.client.node().id;
    let (mut state, mut events) = state(peer.client.clone());
    state.session_navigation(super::Ask {
        ticket: NavigationTicket::FIRST,
        target: NavigationTarget::Address(SessionAddress {
            node,
            session: SessionId::random(),
        }),
    });
    let Event::Sessions(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert!(matches!(answer.result, Err(super::Failure::WrongSession)));
}

#[tokio::test]
async fn same_request_waits_for_matching_connection_ready_without_provider_actions() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let node = peer.client.node().id;
    let (mut state, mut events) = state(peer.client.clone());
    state.clients.clear();
    let session = SessionId::random();
    let ticket = NavigationTicket::FIRST.next().unwrap();
    state.session_navigation(super::Ask {
        ticket,
        target: NavigationTarget::Address(SessionAddress { node, session }),
    });
    assert!(events.try_recv().is_err());
    let connection = connected(&mut state, "reconnected", peer.client.clone());
    let previous = state.clients.insert("reconnected".to_owned(), connection);
    assert!(previous.is_none());
    state.session_connections_changed();
    let Event::Sessions(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert_eq!(answer.ticket, ticket);
    assert_eq!(answer.result.unwrap().host.as_str(), "reconnected");
    assert!(
        matches!(peer.calls.recv().await, Some(Call::GetSession(request)) if request.session == session)
    );
    assert!(peer.calls.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn superseding_a_waiting_link_cancels_only_the_older_metadata_request() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let node = peer.client.node().id;
    let (mut state, mut events) = state(peer.client.clone());
    let first = NavigationTicket::FIRST.next().unwrap();
    let second = first.next().unwrap();
    state.session_navigation(super::Ask {
        ticket: first,
        target: NavigationTarget::Address(SessionAddress {
            node: NodeId::random(),
            session: SessionId::random(),
        }),
    });
    let session = SessionId::random();
    state.session_navigation(super::Ask {
        ticket: second,
        target: NavigationTarget::Address(SessionAddress { node, session }),
    });
    let Event::Sessions(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert_eq!(answer.ticket, second);
    assert!(answer.result.is_ok());
    assert!(
        matches!(peer.calls.recv().await, Some(Call::GetSession(request)) if request.session == session)
    );
    assert!(peer.calls.try_recv().is_err());
    tokio::time::advance(TEST_WAIT).await;
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn metadata_transport_loss_retries_same_address_on_authenticated_reconnect() {
    let node = NodeId::random();
    let mut first = peer::connect_as(peer::Behavior::Disconnect, node).await;
    let (mut state, mut events) = state(first.client.clone());
    let session = SessionId::random();
    let ticket = NavigationTicket::FIRST;
    state.session_navigation(super::Ask {
        ticket,
        target: NavigationTarget::Address(SessionAddress { node, session }),
    });
    assert!(
        matches!(tokio::time::timeout(TEST_WAIT, first.calls.recv()).await.unwrap(),
        Some(Call::GetSession(request)) if request.session == session)
    );
    let mut second = peer::connect_as(peer::Behavior::Accept, node).await;
    state.clients.clear();
    let connection = connected(&mut state, "after-reconnect", second.client.clone());
    let previous = state
        .clients
        .insert("after-reconnect".to_owned(), connection);
    assert!(previous.is_none());
    state.session_connections_changed();
    let Event::Sessions(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert_eq!(answer.ticket, ticket);
    let located = answer.result.unwrap();
    assert_eq!(located.address, SessionAddress { node, session });
    assert_eq!(located.host.as_str(), "after-reconnect");
    assert!(
        matches!(second.calls.recv().await, Some(Call::GetSession(request)) if request.session == session)
    );
    assert!(first.calls.try_recv().is_err());
    assert!(second.calls.try_recv().is_err());
}
