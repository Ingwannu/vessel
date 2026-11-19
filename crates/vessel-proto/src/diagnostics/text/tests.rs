//! The report's text against the fixtures the page's renderer is held to
//! as well (`crate::wire_fixtures`), and the words rule both follow.

use serde_json::json;

use super::*;
use crate::wire_fixtures;

#[test]
fn typescript_fixture_of_a_node_report_renders_as_paseos_text() {
    let sent: DiagnosticReport =
        serde_json::from_str(&wire_fixtures::read("node-diagnostics.json")).unwrap();
    let text = report(&sent);
    assert!(
        wire_fixtures::current("node-diagnostics.txt", &text),
        "node-diagnostics.txt is not what the renderer makes; rerun with VESSEL_WRITE_TS=1:\n{text}"
    );
    assert!(text.starts_with("Vessel diagnostics\n  Collected at: "));
    // Sections apart by a blank line, as Paseo's.
    assert!(text.contains("\n\nDaemon process\n"));
    assert!(text.contains("\n\nTools\n"));
}

#[test]
fn a_report_without_the_new_sections_ends_with_recent_failures() {
    let mut sent: Value =
        serde_json::from_str(&wire_fixtures::read("node-diagnostics.json")).unwrap();
    let _more = sent.as_object_mut().unwrap().remove("more");
    let older: DiagnosticReport = serde_json::from_value(sent).unwrap();
    let text = report(&older);
    assert!(
        text.ends_with("  2026-11-18T20:59:30Z: not found\n"),
        "{text}"
    );
    assert!(!text.contains("System\n"));
}

#[test]
fn words_read_wire_names_tags_and_fields() {
    let word = |value: Value| json_words(&value);
    assert_eq!(word(json!(null)), "none");
    assert_eq!(word(json!(true)), "yes");
    assert_eq!(word(json!("not_checked")), "not checked");
    assert_eq!(word(json!(["a_b", 2])), "a b, 2");
    assert_eq!(word(json!({"state": "available"})), "available");
    assert_eq!(
        word(json!({"state": "unknown", "reason": {"reason": "timed_out", "after": 5}})),
        "unknown (reason: timed out (after: 5))"
    );
    // Untagged: fields by name.
    assert_eq!(word(json!({"b_c": 1, "a": "x_y"})), "a: x y, b c: 1");
    assert_eq!(hundredths(7), "0.07");
    assert_eq!(hundredths(1234), "12.34");
}
