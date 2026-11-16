//! Label fixtures follow the source's trim/blank policy, not book-name parsing.
use super::*;

#[test]
fn unicode_display_labels_are_separate_from_ascii_book_aliases() {
    let label = Label::parse("  우리 머신 😀  ").expect("a Unicode display label");
    assert_eq!(label.as_str(), "우리 머신 😀");
    assert!(crate::machine_book::Name::parse(label.as_str()).is_err());
    let json = serde_json::to_string(&label).expect("serialize the label");
    assert_eq!(
        serde_json::from_str::<Label>(&json).expect("read it"),
        label
    );
}

#[test]
fn source_outer_whitespace_matches_js_including_bom_and_nel() {
    let label = Label::parse("\u{feff}\u{00a0}한글\u{3000}\u{feff}").expect("source trim");
    assert_eq!(label.as_str(), "한글");
    // ECMAScript does not include NEL in its trim set.
    assert_eq!(
        Label::parse(" \u{0085} ")
            .expect("NEL is retained")
            .as_str(),
        "\u{0085}"
    );
    for text in ["", " \t\r\n", "\u{feff}\u{3000}\u{00a0}"] {
        assert_eq!(Label::parse(text), Err(EmptyLabel));
    }
}

#[test]
fn missing_legacy_labels_use_identity_and_explicit_labels_remain_distinct() {
    let node = NodeId::random();
    assert_eq!(Label::or_node(None, node).as_str(), node.to_string());
    let label = Label::parse("My machine").expect("a display label");
    assert_eq!(Label::or_node(Some(&label), node), label);
}
