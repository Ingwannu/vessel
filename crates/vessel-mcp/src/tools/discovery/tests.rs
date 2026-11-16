use super::*;

#[test]
fn omitted_selection_preserves_legacy_until_discovery_is_supported() {
    let args = JsonObject::new();
    assert!(request(&args, false).unwrap().selection().is_none());
    assert_eq!(
        request(&args, true).unwrap().selection(),
        Some(&SessionSelection::shown())
    );
}

#[test]
fn explicit_scope_and_empty_predicates_never_become_legacy() {
    for args in [json!({"scope":"all"}), json!({"labels":{}})] {
        let selected = request(args.as_object().unwrap(), false).unwrap();
        assert!(selected.selection().is_some());
        assert!(vessel_proto::Request::into_call(selected).since().is_none());
    }
    let args = json!({"scope":"all","labels":{" purpose ":" audit "}});
    let selected = request(args.as_object().unwrap(), false).unwrap();
    let selected = selected.selection().unwrap();
    assert_eq!(selected.scope, ListingScope::All);
    let mut pairs = selected.labels.iter();
    let (key, value) = pairs.next().unwrap();
    assert_eq!((key.as_str(), value.as_str()), ("purpose", " audit "));
    assert!(pairs.next().is_none());
}

#[test]
fn invalid_or_excess_selectors_refuse_without_peer_input_in_errors() {
    for args in [
        json!({"scope":null}),
        json!({"scope":"private-canary"}),
        json!({"labels":null}),
        json!({"labels":[]}),
        json!({"labels":{"vessel.private-canary":"x"}}),
        json!({"labels":{"safe":{"private-canary":"x"}}}),
    ] {
        let error = request(args.as_object().unwrap(), false).unwrap_err();
        assert!(!format!("{error:?} {error}").contains("private-canary"));
    }
    let fields = (0..=SessionLabels::MAX)
        .map(|i| (format!("key{i}"), json!("value")))
        .collect::<JsonObject>();
    let args = json!({"labels":fields});
    assert!(request(args.as_object().unwrap(), false).is_err());
}
