//! Alias replacement after resolution must not hydrate, follow or mutate another node.
use super::{next, peer, state};
use crate::{
    Acted, Event,
    sessions::{Access, Binding, Read},
};
use vessel_app_view::{
    machine_book::Name,
    sessions::{Lifecycle, LifecycleAction, LifecycleTarget, SessionAddress},
};
use vessel_proto::{Call, NodeId, SessionId};

fn binding(node: NodeId) -> Binding {
    Binding {
        host: Name::parse("renamed").unwrap(),
        address: SessionAddress {
            node,
            session: SessionId::random(),
        },
    }
}
#[tokio::test]
async fn replaced_alias_refuses_all_passive_access_without_contacting_replacement() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let (mut state, mut events) = state(peer.client.clone());
    let target = binding(NodeId::random());
    for access in [
        Access::Read {
            binding: target.clone(),
            read: Read::Info,
        },
        Access::Read {
            binding: target.clone(),
            read: Read::Conversation,
        },
        Access::Follow(target.clone()),
    ] {
        state.session_access(access);
        let event = next(&mut events).await;
        if let Event::Acted(answer) = event {
            assert_eq!(answer.session, target.address.session);
            assert!(answer.result.is_err());
        } else if let Event::SessionLost { session, .. } = event {
            assert_eq!(session, target.address.session);
        } else {
            panic!("unexpected event: {event:?}");
        }
        assert!(peer.calls.try_recv().is_err());
    }
}

#[tokio::test]
async fn bound_metadata_read_uses_the_authenticated_client_and_exact_session() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let (mut state, mut events) = state(peer.client.clone());
    let target = binding(peer.client.node().id);
    state.session_access(Access::Read {
        binding: target.clone(),
        read: Read::Info,
    });
    let Event::Acted(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert!(matches!(answer.result, Ok(Acted::Session(info)) if info.id == target.address.session));
    assert!(
        matches!(peer.calls.recv().await, Some(Call::GetSession(request)) if request.session == target.address.session)
    );
    assert!(peer.calls.try_recv().is_err());
}

#[tokio::test]
async fn bound_archive_refuses_alias_replacement_before_issuing_any_mutation() {
    let mut peer = peer::connect(peer::Behavior::Accept).await;
    let (state, mut events) = state(peer.client.clone());
    let target = binding(NodeId::random());
    let mut lifecycle = Lifecycle::default();
    let request = lifecycle
        .begin(
            LifecycleTarget {
                host: target.host,
                session: target.address.session,
                node: Some(target.address.node),
            },
            LifecycleAction::Exit,
        )
        .unwrap();
    state.session_lifecycle(&request);
    let Event::SessionLifecycle(answer) = next(&mut events).await else {
        panic!("wrong event")
    };
    assert!(matches!(
        answer.result,
        Err(crate::sessions::lifecycle::Failure::IdentityChanged)
    ));
    assert_eq!(answer.ticket, request.ticket);
    assert!(peer.calls.try_recv().is_err());
}
