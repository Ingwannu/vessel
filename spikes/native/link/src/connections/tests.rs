//! Actor retirement is synchronous and precedes any slow dial or publication.
//! Mirrors Paseo host-runtime.test.ts:751/800/4647 with typed book facts.
use super::*;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::Reach;
use vessel_app_view::connection::{ConnectionTag, MachineKey};
use vessel_app_view::machine_book::{Edit, Name};
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
    within(async {
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
    })
    .await;
}

#[tokio::test]
async fn observed_missing_and_changed_bindings_retire_before_any_dial_or_frame() {
    within(async {
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
        phase(
            "retire the observed changed binding",
            state.reconcile_connections(&[("fixture".parse::<NodeName>().unwrap(), changed)]),
        )
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
        phase(
            "retire the observed absent binding",
            state.reconcile_connections(&[]),
        )
        .await;
        assert!(state.clients.is_empty());
        assert_eq!(*missing.borrow(), Validity::Retired);
        assert!(peer.calls.lock().unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn reload_retirement_keeps_one_epoch_owner_and_disconnect_retires_semantic_facts() {
    within(async {
        let home = Home::new();
        let (mut state, _events) = state(&home);
        let mut peer = peer::connect("fixture").await;
        let client = peer.client.take().unwrap();
        let first = context(&mut state, &client);
        let connected = Connected::new(client.clone(), Vec::new(), binding(), first.clone());
        let validity = connected.validity();
        let _old = state.clients.insert("fixture".into(), connected);
        phase(
            "retire cached admissions on reload",
            state.retire_connections(),
        )
        .await;
        assert!(state.clients.is_empty());
        assert_eq!(*validity.borrow(), Validity::Retired);
        let next = context(&mut state, &client);
        assert_ne!(first.epoch(), next.epoch());
        let connected = Connected::new(client.clone(), Vec::new(), binding(), next);
        let validity = connected.validity();
        let _old = state.clients.insert("fixture".into(), connected.clone());
        peer.task.take().unwrap().abort();
        phase("observe the closed peer transport", client.closed()).await;
        let machine = phase(
            "publish closed session-read facts",
            state.read_sessions("fixture".into(), connected),
        )
        .await;
        assert!(matches!(machine.reach, Reach::Unreachable(_)));
        assert!(state.clients.is_empty());
        assert_eq!(*validity.borrow(), Validity::Retired);
    })
    .await;
}

async fn collect_tags(
    events: &mut tokio::sync::mpsc::Receiver<Event>,
    count: usize,
) -> Vec<ConnectionTag> {
    let mut tags = Vec::new();
    for _ in 0..count {
        let Event::DeviceRetired(tag) = events.recv().await.unwrap() else {
            panic!("only retirement was sent");
        };
        tags.push(tag);
    }
    tags
}

#[tokio::test]
async fn rename_retires_a_stale_destination_before_event_backpressure() {
    within(async {
        const EVENT_CAPACITY: usize = 1;
        for source_present in [false, true] {
            let home = Home::new();
            let (mut state, _original_events) = state(&home);
            let (events, mut receiving) = tokio::sync::mpsc::channel(EVENT_CAPACITY);
            state.events = events;
            let mut source_peer = peer::connect("source").await;
            let client = source_peer.client.take().unwrap();
            let original = context(&mut state, &client);
            let source_connected = Connected::new(client, Vec::new(), binding(), original.clone());
            if source_present {
                let _old = state
                    .clients
                    .insert("fixture".into(), source_connected.clone());
            }
            let mut destination_peer = peer::connect("stale-destination").await;
            let client = destination_peer.client.take().unwrap();
            let replaced = state
                .admissions
                .admit(MachineKey::from_book("destination"), client.node().id)
                .unwrap();
            let destination_connected =
                Connected::new(client, Vec::new(), binding(), replaced.clone());
            let _old = state
                .clients
                .insert("destination".into(), destination_connected.clone());
            phase(
                "fill the retirement event barrier",
                state.events.send(Event::DeviceRetired(replaced.tag())),
            )
            .await
            .unwrap();
            let expected = if source_present {
                vec![replaced.tag(), original.tag()]
            } else {
                vec![replaced.tag()]
            };
            {
                let changing = state.book_connections_applied(Edit::Rename {
                    from: Name::parse("fixture").unwrap(),
                    to: Name::parse("destination").unwrap(),
                });
                tokio::pin!(changing);
                phase("reach blocked retirement publication", async {
                    tokio::select! {
                        biased;
                        () = &mut changing => panic!("the full event queue must block publication"),
                        () = std::future::ready(()) => {}
                    }
                })
                .await;
                // Keep both Connected owners alive so a closed watch sender cannot
                // masquerade as explicit retirement of the observed binding.
                assert_eq!(
                    *destination_connected.validity().borrow(),
                    Validity::Retired
                );
                if source_present {
                    assert_eq!(*source_connected.validity().borrow(), Validity::Retired);
                }
                let _prefilled = receiving.try_recv().unwrap();
                // Scoped futures are cancelled together on a phase failure;
                // there is no detached collector JoinHandle left behind.
                let ((), tags) = tokio::join!(
                    phase("publish ordered retirement tags", changing),
                    phase(
                        "receive all ordered retirement tags",
                        collect_tags(&mut receiving, expected.len())
                    ),
                );
                assert_eq!(tags, expected);
            }
            assert!(matches!(
                state.selected_connection(&replaced),
                Err(vessel_app_view::connection::Refusal::Changed)
            ));
            if source_present {
                let current = &state.clients["destination"];
                assert_eq!(current.client.node().id, original.node());
                assert_ne!(current.context.epoch(), original.epoch());
            } else {
                assert!(state.clients.is_empty());
            }
        }
    })
    .await;
}

#[tokio::test]
async fn an_unchanged_alias_does_not_retire_or_allocate_an_admission() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::connect("fixture").await;
        let client = peer.client.take().unwrap();
        let original = context(&mut state, &client);
        let connected = Connected::new(client, Vec::new(), binding(), original.clone());
        let _old = state.clients.insert("fixture".into(), connected.clone());
        phase(
            "apply the unchanged-name edit",
            state.book_connections_applied(Edit::Rename {
                from: Name::parse("fixture").unwrap(),
                to: Name::parse("fixture").unwrap(),
            }),
        )
        .await;
        assert!(state.clients["fixture"].matches(&original));
        assert_eq!(*connected.validity().borrow(), Validity::Active);
        assert!(events.try_recv().is_err());
    })
    .await;
}
