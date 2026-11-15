//! Real typed RPC tests of session resolution and lifecycle acknowledgement.
mod peer;
use crate::{Connected, Event, State};
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc;
use vessel_app_view::machine_book::Name;
use vessel_app_view::sessions::{
    Lifecycle, LifecycleAction, LifecycleTarget, NavigationTarget, NavigationTicket, SessionAddress,
};
use vessel_proto::{Call, NodeId, SessionId};
fn state(client: vessel_client::NodeClient) -> (State, mpsc::Receiver<Event>) {
    let (events, receiving) = mpsc::channel(8);
    (
        State {
            label: "tests".to_owned(),
            events,
            clients: HashMap::from([(
                "renamed".to_owned(),
                Connected {
                    client,
                    agents: Vec::new(),
                },
            )]),
            follow: None,
            follow_machine: None,
            book_path: crate::machines::BookPath::NoConfiguration,
            workspace: crate::workspace::Hub::default(),
            agents: crate::agents::Hub::default(),
            device_calls: crate::devices::Hub::default(),
            sync_calls: crate::sync::Hub::default(),
            offers: std::sync::Arc::default(),
        },
        receiving,
    )
}
async fn next(events: &mut mpsc::Receiver<Event>) -> Event {
    tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .unwrap()
        .unwrap()
}
#[tokio::test]
async fn address_resolution_survives_alias_rename_and_only_reads_metadata() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let node = peer.client.node().id;
    let (state, mut events) = state(peer.client.clone());
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
#[tokio::test]
async fn another_node_identity_never_falls_back_to_a_similar_saved_name() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let (state, mut events) = state(peer.client.clone());
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
    assert!(matches!(answer.result, Err(super::Failure::Offline)));
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
    let (state, mut events) = state(peer.client.clone());
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
