//! Collection invariants without transport, filesystem or authenticated proof.
use super::*;
use std::path::PathBuf;

fn id(number: u32) -> BookRouteId {
    BookRouteId::new(number).unwrap()
}

fn websocket(number: u32) -> Connection {
    Connection::new(
        id(number),
        ConnectionSpec::WebSocket {
            url: format!("ws://route-{number}.invalid:7077/rpc")
                .parse()
                .unwrap(),
            token: None,
        },
    )
}

fn secure(number: u32, key: u8, credential: &str) -> Connection {
    Connection::new(
        id(number),
        ConnectionSpec::Secure {
            route: Route::Direct(format!("ws://route-{number}.invalid:7077").parse().unwrap()),
            key: NodePublicKey::from_bytes([key; 32]),
            credential: CredentialSource::File(PathBuf::from(credential)),
        },
    )
}

#[test]
fn empty_and_one_over_refuse_but_the_exact_route_budget_is_retained() {
    assert_eq!(
        Connections::new(vec![], id(1), id(1)),
        Err(ConnectionProblem::Empty)
    );
    let maximum = u32::try_from(MAX_BOOK_ROUTES_PER_HOST).unwrap();
    let records: Vec<_> = (1..=maximum).map(websocket).collect();
    let exact = Connections::new(records.clone(), id(17), id(maximum)).unwrap();
    assert_eq!(exact.iter().cloned().collect::<Vec<_>>(), records);
    assert_eq!(exact.preferred(), id(17));
    let over = maximum.checked_add(1).unwrap();
    let records = (1..=over).map(websocket).collect();
    assert_eq!(
        Connections::new(records, id(1), id(over)),
        Err(ConnectionProblem::TooMany)
    );
}

#[test]
fn ids_preference_and_allocation_history_cannot_be_silently_repaired() {
    let first = websocket(1);
    let mut duplicate_id = websocket(2);
    duplicate_id.id = first.id;
    assert_eq!(
        Connections::new(vec![first.clone(), duplicate_id], id(1), id(2)),
        Err(ConnectionProblem::DuplicateId(id(1)))
    );
    let duplicate_spec = Connection::new(id(2), first.specification().clone());
    assert_eq!(
        Connections::new(vec![first, duplicate_spec], id(1), id(2)),
        Err(ConnectionProblem::DuplicateSpecification)
    );
    assert_eq!(
        Connections::new(vec![websocket(2)], id(2), id(1)),
        Err(ConnectionProblem::CounterBehind)
    );
    assert_eq!(
        Connections::new(vec![websocket(2)], id(1), id(3)),
        Err(ConnectionProblem::UnknownPreferred)
    );
    // Earlier removals do not erase allocation history or reuse an id.
    let retained = Connections::new(vec![websocket(2)], id(2), BookRouteId::LAST).unwrap();
    assert_eq!(retained.last_allocated(), BookRouteId::LAST);
    assert_eq!(retained.last_allocated().next(), None);
    assert_eq!(retained.get(id(1)), None);
    assert_eq!(retained.get(id(2)).unwrap().id(), id(2));
}

#[test]
fn secure_routes_share_the_exact_key_and_credential_file_binding() {
    let one = secure(1, 7, "private-a");
    let same = secure(2, 7, "private-a");
    let set = Connections::new(vec![one.clone(), same], id(1), id(2)).unwrap();
    assert_eq!(set.iter().count(), 2);
    for changed in [secure(2, 8, "private-a"), secure(2, 7, "private-b")] {
        assert_eq!(
            Connections::new(vec![one.clone(), changed], id(1), id(2)),
            Err(ConnectionProblem::SecureBindingMismatch)
        );
    }
    // This is configuration only, never permission to fall back to the weaker
    // transport or automatically merge it into a secure identity.
    let mixed = Connections::new(vec![one, websocket(2)], id(2), id(2)).unwrap();
    assert_eq!(mixed.preferred(), id(2));
}

#[test]
fn configuration_diagnostics_do_not_echo_addresses_or_private_references() {
    let canary = "ROUTE_MODEL_PRIVATE_CANARY";
    let spec = ConnectionSpec::WebSocket {
        url: format!("ws://canary.invalid:7077/{canary}")
            .parse()
            .unwrap(),
        token: Some(TokenSource::File(PathBuf::from(canary))),
    };
    let record = Connection::new(id(1), spec);
    let collection = Connections::new(vec![record.clone()], id(1), id(1)).unwrap();
    for text in [format!("{record:?}"), format!("{collection:?}")] {
        assert!(!text.contains(canary));
        assert!(!text.contains("canary.invalid"));
    }
    let error = Connections::new(
        vec![
            record.clone(),
            Connection::new(id(2), record.specification().clone()),
        ],
        id(1),
        id(2),
    )
    .unwrap_err();
    assert!(!error.to_string().contains(canary));
    assert!(!format!("{error:?}").contains(canary));
    assert!(std::error::Error::source(&error).is_none());
}
