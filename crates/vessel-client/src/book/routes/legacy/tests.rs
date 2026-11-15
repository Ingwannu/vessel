//! Conversion is deterministic and never mutates or silently drops legacy data.
use super::*;
use crate::address::SecureTarget;
use crate::book::TokenSource;
use crate::credential::CredentialSource;
use std::path::PathBuf;
use vessel_proto::{NodeId, NodePublicKey};

fn paired() -> BookEntry {
    BookEntry {
        address: NodeAddress::Secure(SecureTarget {
            key: NodePublicKey::from_bytes([7; 32]),
            direct: vec![
                "ws://one.invalid:7077".parse().unwrap(),
                "ws://two.invalid:7077".parse().unwrap(),
            ],
            relays: vec!["wss://relay.invalid/vessel".parse().unwrap()],
        }),
        token: None,
        credential: Some(CredentialSource::File(PathBuf::from("private-credential"))),
        id: Some(NodeId::random()),
    }
}

#[test]
fn secure_direct_then_relay_ids_are_deterministic_without_rewriting_the_entry() {
    let old = paired();
    let before = old.clone();
    let one = Connections::try_from(&old).unwrap();
    assert_eq!(Connections::try_from(&old).unwrap(), one);
    assert_eq!(old, before);
    assert_eq!(one.preferred(), BookRouteId::FIRST);
    assert_eq!(one.last_allocated().get(), 3);
    let records: Vec<_> = one.iter().collect();
    assert_eq!(
        records
            .iter()
            .map(|record| record.id().get())
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    for (record, expected) in records.iter().zip([
        Route::Direct("ws://one.invalid:7077".parse().unwrap()),
        Route::Direct("ws://two.invalid:7077".parse().unwrap()),
        Route::Relay("wss://relay.invalid/vessel".parse().unwrap()),
    ]) {
        let ConnectionSpec::Secure {
            route,
            key,
            credential,
        } = record.specification()
        else {
            panic!("wrong configured transport")
        };
        assert_eq!(route, &expected);
        assert_eq!(key, &NodePublicKey::from_bytes([7; 32]));
        assert_eq!(Some(credential), old.credential.as_ref());
    }
}

#[test]
fn legacy_websocket_and_ssh_keep_their_exact_existing_bindings() {
    let mut ws = BookEntry {
        address: NodeAddress::Ws("ws://bearer.invalid:7077/rpc".parse().unwrap()),
        token: Some(TokenSource::File(PathBuf::from("private-token"))),
        credential: None,
        id: None,
    };
    let connections = Connections::try_from(&ws).unwrap();
    let ConnectionSpec::WebSocket { url, token } =
        connections.get(BookRouteId::FIRST).unwrap().specification()
    else {
        panic!("wrong configured transport")
    };
    assert_eq!(url.to_string(), "ws://bearer.invalid:7077/rpc");
    assert_eq!(token, &ws.token);
    assert_eq!(connections.last_allocated(), BookRouteId::FIRST);
    ws.credential = Some(CredentialSource::File(PathBuf::from("contradiction")));
    assert_eq!(
        Connections::try_from(&ws),
        Err(ConnectionProblem::InvalidLegacyBinding)
    );
    let mut target: crate::address::SshTarget = "ssh://user@ssh.invalid:2222".parse().unwrap();
    target.data_dir = Some("~/.vessel-custom".parse().unwrap());
    let ssh = BookEntry {
        address: NodeAddress::Ssh(target.clone()),
        token: None,
        credential: None,
        id: None,
    };
    let connections = Connections::try_from(&ssh).unwrap();
    assert_eq!(
        connections.get(BookRouteId::FIRST).unwrap().specification(),
        &ConnectionSpec::Ssh(target)
    );
    let mut invalid = ssh;
    invalid.token = ws.token;
    assert_eq!(
        Connections::try_from(&invalid),
        Err(ConnectionProblem::InvalidLegacyBinding)
    );
}

#[test]
fn legacy_expansion_checks_the_same_route_budget_before_new_allocation() {
    let mut entry = paired();
    let NodeAddress::Secure(target) = &mut entry.address else {
        panic!("paired fixture must retain its secure transport")
    };
    target.direct = (0..MAX_BOOK_ROUTES_PER_HOST)
        .map(|n| format!("ws://route-{n}.invalid:7077").parse().unwrap())
        .collect();
    target.relays.clear();
    let exact = Connections::try_from(&entry).unwrap();
    assert_eq!(exact.iter().count(), MAX_BOOK_ROUTES_PER_HOST);
    let NodeAddress::Secure(target) = &mut entry.address else {
        panic!("paired fixture must retain its secure transport")
    };
    target
        .relays
        .push("wss://relay.invalid/vessel".parse().unwrap());
    let before = entry.clone();
    assert_eq!(
        Connections::try_from(&entry),
        Err(ConnectionProblem::TooMany)
    );
    assert_eq!(entry, before);
}

#[test]
fn empty_duplicate_and_missing_credential_are_refused_intact() {
    let mut entry = paired();
    entry.credential = None;
    assert_eq!(
        Connections::try_from(&entry),
        Err(ConnectionProblem::InvalidLegacyBinding)
    );
    entry.credential = Some(CredentialSource::File(PathBuf::from("private")));
    let NodeAddress::Secure(target) = &mut entry.address else {
        panic!("paired fixture must retain its secure transport")
    };
    target.direct.clear();
    target.relays.clear();
    assert_eq!(Connections::try_from(&entry), Err(ConnectionProblem::Empty));
    let NodeAddress::Secure(target) = &mut entry.address else {
        panic!("paired fixture must retain its secure transport")
    };
    let route: vessel_proto::RouteUrl = "ws://duplicate.invalid:7077".parse().unwrap();
    target.direct = vec![route.clone(), route];
    let before = entry.clone();
    assert_eq!(
        Connections::try_from(&entry),
        Err(ConnectionProblem::DuplicateSpecification)
    );
    assert_eq!(entry, before);
}
