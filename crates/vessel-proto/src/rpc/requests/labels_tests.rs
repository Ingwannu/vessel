//! The mutation payload keeps its required target and nonempty map at the edge.

use super::SetLabels;
use crate::SessionId;
use serde_json::json;

#[test]
fn label_mutation_keeps_exact_target_and_validated_map_shape() {
    let session = SessionId::random();
    let body = json!({"session": session, "labels": {"team": "  x=y  "}});
    let request: SetLabels = serde_json::from_value(body.clone()).unwrap();
    assert_eq!(request.session, session);
    assert_eq!(serde_json::to_value(&request).unwrap(), body);
    assert_eq!(request.labels.labels().iter().len(), 1);
}

#[test]
fn label_mutation_requires_both_a_target_and_a_nonempty_patch() {
    let session = SessionId::random();
    for body in [
        json!({}),
        json!({"session": session}),
        json!({"labels": {"team": "x"}}),
        json!({"session": session, "labels": {}}),
        json!({"session": session, "labels": null}),
        json!({"session": "not-a-session", "labels": {"team": "x"}}),
    ] {
        assert!(serde_json::from_value::<SetLabels>(body).is_err());
    }
}
