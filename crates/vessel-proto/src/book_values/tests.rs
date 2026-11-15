//! Compatibility of the shared client-book values, not a format parser.
use super::*;
use crate::NodeId;

#[test]
fn the_existing_color_catalogue_and_badge_default_do_not_change() {
    assert_eq!(
        Color::ALL,
        [
            Color::None,
            Color::Violet,
            Color::Sky,
            Color::Emerald,
            Color::Orange,
            Color::Pink,
            Color::Indigo,
            Color::Teal,
            Color::Red,
            Color::Amber,
            Color::Blue,
        ]
    );
    assert_eq!(Color::default(), Color::None);
    assert_eq!(BadgePreference::default(), BadgePreference::Automatic);
}

#[test]
fn all_source_trim_characters_and_unicode_serialize_without_ascii_alias_rules() {
    for whitespace in [
        '\u{0009}', '\u{000a}', '\u{000b}', '\u{000c}', '\u{000d}', '\u{0020}', '\u{00a0}',
        '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
        '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}',
        '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
    ] {
        let text = format!("{whitespace}우리 머신 😀{whitespace}");
        let label = Label::parse(&text).unwrap();
        assert_eq!(label.as_str(), "우리 머신 😀");
        assert_eq!(Label::parse(&whitespace.to_string()), Err(EmptyLabel));
        let encoded = serde_json::to_string(&label).unwrap();
        assert_eq!(serde_json::from_str::<Label>(&encoded).unwrap(), label);
    }
    assert_eq!(Label::parse(" \u{0085} ").unwrap().as_str(), "\u{0085}");
}

#[test]
fn the_missing_label_falls_back_only_to_typed_identity_and_blank_faults_are_safe() {
    let node = NodeId::random();
    assert_eq!(Label::or_node(None, node).as_str(), node.to_string());
    let label = Label::parse("My machine").unwrap();
    assert_eq!(Label::or_node(Some(&label), node), label);
    let error = Label::parse(" \t\r\n ").unwrap_err();
    assert_eq!(error, EmptyLabel);
    assert_eq!(
        error.to_string(),
        "a machine display label must not be blank"
    );
}
