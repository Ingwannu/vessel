//! Fictional discovery requests are decoded here; tests carry the typed policy.

use super::*;
use crate::{Call, ListSessions, Request as _};

#[test]
fn null_is_the_unchanged_legacy_request_and_present_scope_is_explicit() {
    let legacy = ListSessions::default();
    assert_eq!(serde_json::to_string(&legacy).unwrap(), "null");
    assert_eq!(
        serde_json::from_str::<ListSessions>("null").unwrap(),
        legacy
    );
    assert_eq!(legacy.selection(), None);
    for (text, selected) in [
        (r#"{"scope":"shown"}"#, SessionSelection::shown()),
        (r#"{"scope":"all","labels":{}}"#, SessionSelection::all()),
    ] {
        let request: ListSessions = serde_json::from_str(text).unwrap();
        assert_eq!(request.selection(), Some(&selected));
        assert_eq!(
            serde_json::from_str::<ListSessions>(&serde_json::to_string(&request).unwrap())
                .unwrap(),
            ListSessions::selected(selected)
        );
    }
}

#[test]
fn a_present_selection_without_a_known_scope_or_valid_labels_is_invalid() {
    for text in [
        "{}",
        r#"{"labels":{"env":"prod"}}"#,
        r#"{"scope":"background"}"#,
        r#"{"scope":true}"#,
        r#"{"scope":null}"#,
        r#"{"scope":"all","labels":{"":"value"}}"#,
    ] {
        assert!(
            serde_json::from_str::<ListSessions>(text).is_err(),
            "{text}"
        );
    }
    let labels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
    let request: ListSessions =
        serde_json::from_str(r#"{"scope":"shown","labels":{"env":"prod"}}"#).unwrap();
    assert_eq!(
        request,
        ListSessions::selected(SessionSelection::shown().with_labels(labels))
    );
}

#[test]
fn only_a_present_selection_contributes_the_pending_feature_requirement() {
    assert_eq!(crate::lifecycle_version::SINCE, None);
    assert_eq!(
        ListSessions::default().into_call().since(),
        ListSessions::METHOD.since()
    );
    for selection in [SessionSelection::shown(), SessionSelection::all()] {
        let call = ListSessions::selected(selection).into_call();
        assert!(matches!(call, Call::ListSessions(_)));
        assert_eq!(call.since(), None);
        assert_eq!(call.required_protocol(), None);
    }
}
