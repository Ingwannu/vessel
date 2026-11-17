//! The usage section and the summary's sidebar flag (ADR 0107): an older
//! document reads as the defaults and is not rewritten for them; each
//! change writes only its own field.
use super::*;
use crate::wire::{UsageDisplay, UsagePins, UsagePreferences};

fn read(text: &str) -> PrefsDoc {
    match PrefsDoc::decode(text.as_bytes()).unwrap() {
        Decoded::Current(doc) => doc,
        Decoded::Newer { version, .. } => panic!("read as newer ({version})"),
    }
}
fn written(doc: &PrefsDoc) -> String {
    let text = String::from_utf8(doc.encode().unwrap()).unwrap();
    text.chars().filter(|c| !c.is_whitespace()).collect()
}
const NODE: &str = "0d4b6a35-37d2-4d5c-9f3e-6f1a2b7c8d90";

#[test]
fn a_document_without_the_section_reads_as_used_no_host_no_pins() {
    for text in ["{}", r#"{"version":1,"sidebar":{"group":"status"}}"#] {
        let doc = read(text);
        let values = doc.values();
        assert_eq!(values.usage, UsagePreferences::default(), "{text}");
        assert_eq!(values.usage.display, UsageDisplay::Used);
        assert!(values.usage.host.is_none() && values.usage.pins.is_none());
        assert!(!values.sidebar.usage, "the summary starts off, as Paseo's");
        assert!(!written(&doc).contains("usage"), "{text}");
    }
}

#[test]
fn each_change_writes_only_its_own_field() {
    let doc = read("{}");
    let shown = doc.changed(&PreferenceChange::SidebarUsage(true)).unwrap();
    assert!(written(&shown).contains(r#""sidebar":{"usage":true}"#));
    let remaining = shown
        .changed(&PreferenceChange::UsageDisplay(UsageDisplay::Remaining))
        .unwrap();
    assert!(written(&remaining).contains(r#""usage":{"display":"remaining"}"#));
    let node: NodeId = NODE.parse().unwrap();
    let hosted = remaining
        .changed(&PreferenceChange::UsageHost(Some(node)))
        .unwrap();
    let values = read(&written(&hosted)).values();
    assert_eq!(values.usage.display, UsageDisplay::Remaining);
    assert_eq!(values.usage.host, Some(node));
    assert!(values.sidebar.usage);
}

#[test]
fn an_empty_pin_list_pins_nothing_and_null_follows_the_defaults() {
    let doc = read("{}");
    let none = doc
        .changed(&PreferenceChange::UsagePins(Some(UsagePins::empty())))
        .unwrap();
    assert!(written(&none).contains(r#""pins":[]"#));
    assert_eq!(
        read(&written(&none)).values().usage.pins,
        Some(UsagePins::empty())
    );
    let defaults = none.changed(&PreferenceChange::UsagePins(None)).unwrap();
    assert!(written(&defaults).contains(r#""pins":null"#));
    assert_eq!(read(&written(&defaults)).values().usage.pins, None);
}

#[test]
fn an_unreadable_usage_field_reads_as_its_default_and_is_kept() {
    let text = r#"{"usage":{"display":"sideways","host":"not-a-node","pins":[{"source":"saved-codex","window":"weekly"}]}}"#;
    let doc = read(text);
    let values = doc.values();
    assert_eq!(values.usage.display, UsageDisplay::Used);
    assert_eq!(values.usage.host, None);
    assert_eq!(values.usage.pins.map(|pins| pins.items().len()), Some(1));
    let rewritten = written(&doc);
    assert!(rewritten.contains(r#""display":"sideways""#));
    assert!(rewritten.contains(r#""host":"not-a-node""#));
}
