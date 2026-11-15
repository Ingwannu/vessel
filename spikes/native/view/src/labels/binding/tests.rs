//! Alias reuse cannot transfer one host's cached colors to another node.

use super::*;
use crate::{
    Machine,
    labels::{Color, HostKey, Label},
};

fn previous(key: HostKey) -> Host {
    Host {
        key,
        route: Name::parse("old-alias").unwrap(),
        state: HostState::Online,
        catalog: Catalog::snapshot([Label {
            name: "Ready".parse().unwrap(),
            color: Color::Violet,
        }]),
    }
}
fn machine(name: &str, node: Option<HostKey>, reach: Reach) -> Machine {
    Machine {
        name: name.to_owned(),
        node,
        reach,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: None,
        version: None,
    }
}
fn fleet(machines: Vec<Machine>) -> Fleet {
    Fleet {
        machines,
        read_at: None,
    }
}

#[test]
fn alias_rename_keeps_catalog_but_rebinding_to_a_different_node_does_not() {
    let old = HostKey::random();
    let new = HostKey::random();
    let before = previous(old);
    let renamed = bind_hosts(
        std::slice::from_ref(&before),
        &fleet(vec![machine("new-alias", Some(old), Reach::Online)]),
    );
    assert_eq!(renamed.hosts.first().unwrap().catalog, before.catalog);
    assert_eq!(renamed.hosts.first().unwrap().route.as_str(), "new-alias");
    let rebound = bind_hosts(
        &[before],
        &fleet(vec![machine("old-alias", Some(new), Reach::Online)]),
    );
    assert_eq!(rebound.hosts.first().unwrap().key, new);
    assert_eq!(rebound.hosts.first().unwrap().catalog.labels().count(), 0);
}

#[test]
fn disconnected_known_alias_retains_colors_and_an_unknown_alias_stays_unresolved() {
    let key = HostKey::random();
    let bound = bind_hosts(
        &[previous(key)],
        &fleet(vec![
            machine("old-alias", None, Reach::Unreachable("offline".into())),
            machine("never-answered", None, Reach::Unreachable("offline".into())),
        ]),
    );
    assert_eq!(bound.hosts.first().unwrap().key, key);
    assert_eq!(bound.hosts.first().unwrap().state, HostState::Offline);
    assert_eq!(bound.hosts.first().unwrap().catalog.labels().count(), 1);
    assert_eq!(
        bound.unresolved,
        vec![Name::parse("never-answered").unwrap()]
    );
}

#[test]
fn two_routes_to_one_node_share_one_catalog_and_prefer_the_answered_route() {
    let key = HostKey::random();
    for machines in [
        vec![
            machine("offline", Some(key), Reach::Unreachable("offline".into())),
            machine("online", Some(key), Reach::Online),
        ],
        vec![
            machine("online", Some(key), Reach::Online),
            machine("offline", Some(key), Reach::Unreachable("offline".into())),
        ],
    ] {
        let bound = bind_hosts(&[previous(key)], &fleet(machines));
        assert_eq!(bound.hosts.len(), 1);
        assert_eq!(bound.hosts.first().unwrap().route.as_str(), "online");
        assert_eq!(bound.hosts.first().unwrap().catalog.labels().count(), 1);
    }
}

#[test]
fn malformed_legacy_alias_is_reported_without_inventing_catalog_ownership() {
    let bound = bind_hosts(
        &[],
        &fleet(vec![machine(
            "bad alias",
            Some(HostKey::random()),
            Reach::Online,
        )]),
    );
    assert_eq!(bound.error, Some(Failure::Load));
    assert!(bound.hosts.is_empty());
}

#[test]
fn unresolved_alias_never_duplicates_an_answered_route_in_either_order() {
    let key = HostKey::random();
    for machines in [
        vec![
            machine("new-alias", Some(key), Reach::Online),
            machine("old-alias", None, Reach::Unreachable("offline".into())),
        ],
        vec![
            machine("old-alias", None, Reach::Unreachable("offline".into())),
            machine("new-alias", Some(key), Reach::Online),
        ],
    ] {
        let bound = bind_hosts(&[previous(key)], &fleet(machines));
        assert_eq!(bound.hosts.len(), 1);
        assert_eq!(bound.hosts.first().unwrap().route.as_str(), "new-alias");
        assert_eq!(bound.hosts.first().unwrap().catalog.labels().count(), 1);
    }
}

#[test]
fn capability_gate_preserves_loaded_authority_only_for_a_capable_connected_peer() {
    let version = ProtocolVersion::new(23);
    let mut peer = machine("peer", Some(HostKey::random()), Reach::Online);
    peer.protocol = Some(version);
    assert_eq!(
        availability(&peer, Some(HostState::Online), None),
        HostState::Unsupported
    );
    assert_eq!(
        availability(&peer, Some(HostState::Online), Some(version)),
        HostState::Online
    );
    assert_eq!(
        availability(&peer, Some(HostState::Failed(Failure::Load)), Some(version)),
        HostState::Failed(Failure::Load)
    );
    assert_eq!(
        availability(&peer, Some(HostState::Offline), Some(version)),
        HostState::Loading
    );
    peer.protocol = None;
    assert_eq!(
        availability(&peer, Some(HostState::Online), Some(version)),
        HostState::Unsupported
    );
    peer.reach = Reach::Unreachable("offline".into());
    assert_eq!(
        availability(&peer, Some(HostState::Online), Some(version)),
        HostState::Offline
    );
}
