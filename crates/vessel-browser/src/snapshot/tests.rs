//! Independently authored graph fixtures; these do not emulate a working browser.
use super::*;
use serde_json::{Value, json};
use vessel_proto::browser::{MAX_PAGE_TEXT_BYTES, TextCompleteness};

fn element(depth: u8, role: &str, name: &str, reference: Option<&str>) -> Value {
    json!({"kind":"element","depth":depth,"role":role,"name":name,"reference":reference,
        "fingerprint":{"tag":"input","id":"own-fixture","input_type":"text","name":"field","value":"synthetic-private-value"},
        "state":{"checked":null,"expanded":null,"selected":null,"disabled":false,"readonly":false,"level":null}})
}
fn encoded(nodes: Vec<Value>, visited: usize, truncated: bool) -> String {
    let nodes = Value::Array(nodes);
    json!({"document":{"address":"","title":""},"nodes":nodes,"visited":visited,"frames":0,"truncated":truncated}).to_string()
}

#[test]
fn document_content_is_required_redacted_and_shares_the_graph_text_budget() {
    let source = encoded(vec![element(0, "document", "Own", None)], 1, false);
    let mut input: Value = serde_json::from_str(&source).unwrap();
    input["document"] =
        json!({"address":"data:display-only-private-fixture","title":"private-title-fixture"});
    let mut refs = References::default();
    let namespace = refs.begin_snapshot().unwrap();
    let snapshot = finish(&input.to_string(), namespace, &mut refs).unwrap();
    assert_eq!(
        snapshot.document.address.as_str(),
        "data:display-only-private-fixture"
    );
    assert!(!format!("{snapshot:?}").contains("private-fixture"));
    assert!(!format!("{snapshot:?}").contains("private-title-fixture"));

    input["document"]["address"] = json!("x".repeat(MAX_PAGE_TEXT_BYTES));
    let namespace = refs.begin_snapshot().unwrap();
    assert!(matches!(
        finish(&input.to_string(), namespace, &mut refs),
        Err(SnapshotError::Graph)
    ));
    let _removed = input.as_object_mut().unwrap().remove("document");
    let namespace = refs.begin_snapshot().unwrap();
    assert!(matches!(
        finish(&input.to_string(), namespace, &mut refs),
        Err(SnapshotError::Decode(_))
    ));
}

#[test]
fn rendered_escaping_cannot_spend_the_document_metadata_budget_again() {
    const TITLE_BYTES: usize = 60_000;
    const TEXT_BYTES: usize = 10_000;
    let source = encoded(
        vec![json!({"kind":"text","depth":0,"text":"\0".repeat(TEXT_BYTES)})],
        1,
        false,
    );
    let mut input: Value = serde_json::from_str(&source).unwrap();
    input["document"]["title"] = json!("x".repeat(TITLE_BYTES));
    let mut refs = References::default();
    let namespace = refs.begin_snapshot().unwrap();
    let snapshot = finish(&input.to_string(), namespace, &mut refs).unwrap();
    assert_eq!(snapshot.content.completeness, TextCompleteness::Truncated);
    assert!(
        snapshot
            .content
            .text
            .as_str()
            .len()
            .saturating_add(TITLE_BYTES)
            <= MAX_PAGE_TEXT_BYTES
    );
    assert!(
        snapshot
            .content
            .text
            .as_str()
            .contains("Snapshot truncated.")
    );
}
#[test]
fn own_graph_renders_escaped_lines_and_keeps_private_fingerprints_out_of_debug() {
    let mut refs = References::default();
    let namespace = refs.begin_snapshot().unwrap();
    let reference = namespace.at(0).unwrap();
    let mut checkbox = element(1, "checkbox", "Agreement", Some(reference.as_str()));
    checkbox["state"]["checked"] = json!("false");
    let input = encoded(
        vec![
            element(0, "document", "Own fixture", None),
            checkbox,
            json!({"kind":"text","depth":1,"text":"x\n- button \"injected\""}),
        ],
        3,
        false,
    );
    let snapshot = finish(&input, namespace, &mut refs).unwrap();
    assert_eq!(
        snapshot.content.text.as_str(),
        "- document \"Own fixture\"\n  - checkbox \"Agreement\" [checked=false ref=@e1]\n  - text: \"x\\n- button \\\"injected\\\"\""
    );
    assert_eq!(snapshot.content.completeness, TextCompleteness::Whole);
    assert_eq!(snapshot.stats.references.get(), 1);
    assert_eq!(
        usize::try_from(snapshot.stats.text_bytes.get()).unwrap(),
        snapshot.content.text.as_str().len()
    );
    assert!(
        !format!("{:?}", refs.resolve(&reference).unwrap()).contains("synthetic-private-value")
    );
    assert!(!format!("{snapshot:?}").contains("Agreement"));
}
#[test]
fn malformed_or_late_result_clears_all_previous_refs_and_consumes_its_range() {
    let mut refs = References::default();
    let first = refs.begin_snapshot().unwrap();
    let old = first.at(0).unwrap();
    let input = encoded(
        vec![element(0, "button", "Old", Some(old.as_str()))],
        1,
        false,
    );
    let _retained = finish(&input, first, &mut refs).unwrap();
    let second = refs.begin_snapshot().unwrap();
    assert!(refs.resolve(&old).is_err());
    assert_ne!(first.at(0).unwrap(), second.at(0).unwrap());
    assert!(finish(&input, second, &mut refs).is_err());
    assert!(refs.resolve(&old).is_err());
    // Even a valid old result must not repopulate refs after another traversal began.
    assert!(finish(&input, first, &mut refs).is_err());
    let third = refs.begin_snapshot().unwrap();
    assert!(third.at(0).unwrap().as_str() == "@e1001");
    let duplicated = third.at(0).unwrap();
    let input = encoded(
        vec![
            element(0, "button", "First", Some(duplicated.as_str())),
            element(1, "button", "Duplicate", Some(duplicated.as_str())),
        ],
        2,
        false,
    );
    assert!(finish(&input, third, &mut refs).is_err());
    assert!(refs.resolve(&duplicated).is_err());
}
#[test]
fn excess_graph_item_is_refused_before_its_payload_and_bad_depth_is_not_empty_success() {
    let item = json!({"kind":"text","depth":0,"text":""}).to_string();
    let mut items = vec![item; MAX_SNAPSHOT_NODES];
    items.push("{\"not_a_node\":\"must_not_decode\"}".to_owned());
    let input = format!(
        "{{\"nodes\":[{}],\"visited\":1501,\"frames\":0,\"truncated\":false}}",
        items.join(",")
    );
    let mut refs = References::default();
    let namespace = refs.begin_snapshot().unwrap();
    let error = finish(&input, namespace, &mut refs).unwrap_err();
    match error {
        SnapshotError::Decode(error) => assert!(error.to_string().contains("node limit")),
        other @ (SnapshotError::Reference(_)
        | SnapshotError::Graph
        | SnapshotError::Render(_)
        | SnapshotError::Evaluation(_)) => panic!("{other}"),
    }
    for input in [
        encoded(vec![], 0, false),
        encoded(vec![element(1, "button", "No root", None)], 1, false),
        encoded(
            vec![
                element(0, "document", "Root", None),
                element(MAX_SNAPSHOT_DEPTH + 1, "button", "Deep", None),
            ],
            2,
            true,
        ),
    ] {
        let namespace = refs.begin_snapshot().unwrap();
        assert!(finish(&input, namespace, &mut refs).is_err());
    }
}
#[test]
fn escaped_unicode_output_reserves_a_complete_visible_truncation_marker() {
    const CHUNK: &str = "\0한";
    let mut refs = References::default();
    let namespace = refs.begin_snapshot().unwrap();
    let text = CHUNK.repeat(MAX_PAGE_TEXT_BYTES.checked_div(CHUNK.len()).unwrap());
    let input = encoded(vec![json!({"kind":"text","depth":0,"text":text})], 1, false);
    let snapshot = finish(&input, namespace, &mut refs).unwrap();
    assert_eq!(snapshot.content.completeness, TextCompleteness::Truncated);
    assert!(snapshot.content.text.as_str().len() <= MAX_PAGE_TEXT_BYTES);
    assert_eq!(
        snapshot.content.text.as_str(),
        "- text: \"Snapshot truncated.\""
    );
    let namespace = refs.begin_snapshot().unwrap();
    let input = encoded(vec![element(0, "document", "Root", None)], 1, true);
    let snapshot = finish(&input, namespace, &mut refs).unwrap();
    assert!(
        snapshot
            .content
            .text
            .as_str()
            .ends_with("\n- text: \"Snapshot truncated.\"")
    );
}

#[test]
fn a_late_graph_without_any_ref_cannot_bypass_the_namespace_fence() {
    let mut refs = References::default();
    let old = refs.begin_snapshot().unwrap();
    let _new = refs.begin_snapshot().unwrap();
    let input = encoded(
        vec![element(0, "document", "Old private page", None)],
        1,
        false,
    );
    assert!(matches!(
        finish(&input, old, &mut refs),
        Err(SnapshotError::Reference(ReferenceError::Stale))
    ));
}

#[test]
fn changed_form_value_invalidates_an_otherwise_identical_binding() {
    let value = element(0, "textbox", "Own", Some("@e1"));
    let node: Node = serde_json::from_value(value).unwrap();
    let Node::Element { fingerprint, .. } = node else {
        panic!("element fixture")
    };
    let original = Binding(fingerprint.clone());
    let same = Binding(fingerprint.clone());
    assert!(original.same_fingerprint(&same));
    let mut changed = fingerprint;
    changed.value =
        vessel_proto::browser::PageText::try_from("changed-by-page".to_owned()).unwrap();
    assert!(!original.same_fingerprint(&Binding(changed)));
}
