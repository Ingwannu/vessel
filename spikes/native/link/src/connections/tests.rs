//! Actor retirement is synchronous and precedes any slow dial or publication.
//! Mirrors Paseo host-runtime.test.ts:751/800/4647 with typed book facts.
use super::*;
use crate::testing::{Home, peer, state};
use vessel_app_view::Reach;
use vessel_app_view::devices::MachineKey;
use vessel_client::book::{NodeName, TokenSource};

fn binding() -> BookEntry {
    BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    }
}
fn context(state: &mut State, client: &NodeClient) -> ConnectionContext {
    state
        .admissions
        .admit(MachineKey::from_book("fixture"), client.node().id)
        .unwrap()
}
#[tokio::test]
async fn only_learning_the_already_authenticated_pin_keeps_the_admission() {
    let home = Home::new();
    let (mut state, _events) = state(&home);
    let mut peer = peer::connect("fixture").await;
    let client = peer.client.take().unwrap();
    let context = context(&mut state, &client);
    let mut connected = Connected::new(client, Vec::new(), binding(), context.clone());
    assert!(connected.retains(&binding()));
    let learned = BookEntry {
        id: Some(context.node()),
        ..binding()
    };
    assert!(connected.retains(&learned));
    assert!(connected.matches(&context));
    for changed in [
        binding(), // removing a learned pin is a replacement too
        BookEntry {
            id: Some(vessel_proto::NodeId::random()),
            ..learned.clone()
        },
        BookEntry {
            address: "ws://127.0.0.1:1/rpc".parse().unwrap(),
            ..learned.clone()
        },
        BookEntry {
            token: Some(TokenSource::File("synthetic-token-path".into())),
            ..learned
        },
    ] {
        assert!(!connected.retains(&changed));
    }
    assert!(peer.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn observed_missing_and_changed_bindings_retire_before_any_dial_or_frame() {
    let home = Home::new();
    let (mut state, _events) = state(&home);
    let mut peer = peer::connect("fixture").await;
    let client = peer.client.take().unwrap();
    let context = context(&mut state, &client);
    let connected = Connected::new(client, Vec::new(), binding(), context.clone());
    let validity = connected.validity();
    let _old = state.clients.insert("fixture".into(), connected);
    let changed = BookEntry {
        address: "ws://127.0.0.1:1/rpc".parse().unwrap(),
        ..binding()
    };
    state
        .reconcile_connections(&[("fixture".parse::<NodeName>().unwrap(), changed)])
        .await;
    assert!(state.clients.is_empty());
    assert_eq!(*validity.borrow(), Validity::Retired);
    // Observing A again cannot revive its retired generation.
    let next = state
        .admissions
        .admit(context.machine().clone(), context.node())
        .unwrap();
    assert_ne!(context.epoch(), next.epoch());
    let mut replacement = peer::connect("replacement").await;
    let client = replacement.client.take().unwrap();
    let next = state
        .admissions
        .admit(MachineKey::from_book("fixture"), client.node().id)
        .unwrap();
    let _old = state.clients.insert(
        "fixture".into(),
        Connected::new(client, Vec::new(), binding(), next),
    );
    let missing = state.clients["fixture"].validity();
    state.reconcile_connections(&[]).await;
    assert!(state.clients.is_empty());
    assert_eq!(*missing.borrow(), Validity::Retired);
    assert!(peer.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn reload_retirement_keeps_one_epoch_owner_and_disconnect_retires_semantic_facts() {
    let home = Home::new();
    let (mut state, _events) = state(&home);
    let mut peer = peer::connect("fixture").await;
    let client = peer.client.take().unwrap();
    let first = context(&mut state, &client);
    let connected = Connected::new(client.clone(), Vec::new(), binding(), first.clone());
    let validity = connected.validity();
    let _old = state.clients.insert("fixture".into(), connected);
    state.retire_connections().await;
    assert!(state.clients.is_empty());
    assert_eq!(*validity.borrow(), Validity::Retired);
    let next = context(&mut state, &client);
    assert_ne!(first.epoch(), next.epoch());
    let connected = Connected::new(client.clone(), Vec::new(), binding(), next);
    let validity = connected.validity();
    let _old = state.clients.insert("fixture".into(), connected.clone());
    peer.task.take().unwrap().abort();
    tokio::time::timeout(crate::testing::DEADLINE, client.closed())
        .await
        .unwrap();
    let machine = state.read_sessions("fixture".into(), connected).await;
    assert!(matches!(machine.reach, Reach::Unreachable(_)));
    assert!(state.clients.is_empty());
    assert_eq!(*validity.borrow(), Validity::Retired);
}
