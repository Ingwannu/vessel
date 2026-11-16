use super::*;
use crate::prefs::{PreferenceChange, Preferences, Theme};

fn key(value: &str) -> Key {
    value.parse().unwrap()
}
fn item(value: &str, visible: bool) -> Item {
    Item {
        key: key(value),
        visible: visible.into(),
    }
}
fn entries(items: Vec<Item>) -> Entries {
    items.try_into().unwrap()
}
fn keys(items: &[Item]) -> Vec<&str> {
    items.iter().map(|item| item.key.as_str()).collect()
}

#[test]
fn defaults_match_both_reference_bands() {
    let blank = Entries::default();
    let header = blank.resolve(Section::Header, &[key("plugin:notes:inbox")]);
    assert_eq!(
        keys(&header),
        [
            "new-workspace",
            "history",
            "search",
            "schedules",
            "plugin:notes:inbox"
        ]
    );
    assert!(header.iter().all(|item| item.visible == Visibility::Shown));
    assert_eq!(
        blank.resolve(Section::Footer, &[]),
        vec![item("usage", false)]
    );
}

#[test]
fn stored_first_wins_unavailable_hidden_ignored_and_new_items_append() {
    let previous = entries(vec![
        item("plugin:gone:inbox", false),
        item("schedules", false),
        item("schedules", true),
        item("bogus", true),
        item("plugin:notes:inbox", false),
        item("usage", true),
    ]);
    let header = previous.resolve(
        Section::Header,
        &[key("plugin:notes:inbox"), key("plugin:notes:inbox")],
    );
    assert_eq!(
        keys(&header),
        [
            "schedules",
            "plugin:notes:inbox",
            "new-workspace",
            "history",
            "search"
        ]
    );
    assert_eq!(header[0].visible, Visibility::Hidden);
    assert_eq!(header[1].visible, Visibility::Hidden);
    assert_eq!(
        previous.resolve(Section::Footer, &[]),
        vec![item("usage", true)]
    );
}

#[test]
fn edits_preserve_absent_keys_in_reference_interleaving_and_reconnect_order() {
    let old = entries(vec![
        item("search", true),
        item("plugin:gone:row", false),
        item("history", true),
        item("history", false),
    ]);
    let available = old.resolve(Section::Header, &[]);
    let next = old
        .moved(&available, &key("history"), Direction::Up)
        .unwrap();
    assert_eq!(
        keys(next.items()),
        [
            "history",
            "plugin:gone:row",
            "search",
            "new-workspace",
            "schedules"
        ]
    );
    let connected = next.resolve(Section::Header, &[key("plugin:gone:row")]);
    assert_eq!(
        keys(&connected),
        [
            "history",
            "plugin:gone:row",
            "search",
            "new-workspace",
            "schedules"
        ]
    );
    assert_eq!(connected[1].visible, Visibility::Hidden);
    let hidden = next
        .visibility(&connected, &key("history"), Visibility::Hidden)
        .unwrap();
    assert_eq!(hidden.items()[0], item("history", false));
    assert_eq!(hidden.items()[1], item("plugin:gone:row", false));
}

#[test]
fn movement_boundaries_and_missing_target_do_not_change_display_order() {
    let blank = Entries::default();
    let all = blank.resolve(Section::Header, &[]);
    for (key, direction) in [
        (key("new-workspace"), Direction::Up),
        (key("schedules"), Direction::Down),
        (key("absent"), Direction::Down),
    ] {
        assert_eq!(blank.moved(&all, &key, direction).unwrap().items(), all);
    }
    let down = blank.moved(&all, &key("history"), Direction::Down).unwrap();
    assert_eq!(
        keys(down.items()),
        ["new-workspace", "search", "history", "schedules"]
    );
}

#[test]
fn persisted_bands_are_independent_and_default_document_stays_compatible() {
    let before = Preferences::default();
    assert!(
        !serde_json::to_string(&before)
            .unwrap()
            .contains("sidebar_navigation")
    );
    let change = PreferenceChange::SidebarNavigation {
        section: Section::Footer,
        entries: entries(vec![item("usage", true), item("plugin:gone:row", false)]),
    };
    let next = before.changed(&change).unwrap();
    assert_eq!(next.appearance.theme, Theme::System);
    assert!(
        next.sidebar_navigation
            .entries(Section::Header)
            .items()
            .is_empty()
    );
    let json = serde_json::to_string(&next).unwrap();
    assert!(json.contains(r#""visible":true"#));
    assert_eq!(serde_json::from_str::<Preferences>(&json).unwrap(), next);
    assert_eq!(
        serde_json::from_str::<PreferenceChange>(&serde_json::to_string(&change).unwrap()).unwrap(),
        change
    );
    let same = key("plugin:notes:row");
    let header = next
        .sidebar_navigation
        .entries(Section::Header)
        .resolve(Section::Header, std::slice::from_ref(&same));
    let footer = next
        .sidebar_navigation
        .entries(Section::Footer)
        .resolve(Section::Footer, &[same]);
    assert_eq!(header.last().unwrap().visible, Visibility::Shown);
    assert_eq!(footer.last().unwrap().visible, Visibility::Shown);
}

#[test]
fn persisted_input_and_growth_are_bounded_without_dropping_old_choices() {
    for invalid in [String::new(), "x\n".to_owned(), "한".repeat(MAX_KEY_BYTES)] {
        assert!(invalid.parse::<Key>().is_err());
    }
    assert!("x".repeat(MAX_KEY_BYTES).parse::<Key>().is_ok());
    let full = entries(
        (0..MAX_ENTRIES)
            .map(|i| item(&format!("plugin:offline:{i}"), false))
            .collect(),
    );
    assert!(Entries::try_from(vec![item("history", true); MAX_ENTRIES + 1]).is_err());
    let available = full.resolve(Section::Header, &[]);
    assert_eq!(
        full.visibility(&available, &key("history"), Visibility::Hidden),
        Err(TooMany)
    );
    assert_eq!(full.items().len(), MAX_ENTRIES);
}

#[test]
fn settings_and_navigation_share_complete_prepared_en_ko_labels() {
    use crate::localization::{ENGLISH, Locale, Translator};
    let en = Translator::get(ENGLISH).unwrap().sidebar();
    let ko = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .sidebar();
    assert_eq!(
        Builtin::ALL.map(|item| en.builtin(item)),
        ["New workspace", "Sessions", "Search", "Schedules", "Usage"]
    );
    assert_eq!(
        Builtin::ALL.map(|item| ko.builtin(item)),
        ["새 작업 공간", "세션", "검색", "일정", "사용량"]
    );
    for words in [en, ko] {
        for section in [Section::Header, Section::Footer] {
            assert!(!words.band_title(section).is_empty());
            assert!(!words.band_description(section).is_empty());
        }
        assert!(!words.navigation_limit().is_empty());
    }
}

#[test]
fn malformed_band_does_not_erase_the_other_band() {
    let prefs:Preferences=serde_json::from_str(r#"{"sidebar_navigation":{"header":[{"key":"","visible":true}],"footer":[{"key":"usage","visible":true}]}}"#).unwrap();
    assert!(
        prefs
            .sidebar_navigation
            .entries(Section::Header)
            .items()
            .is_empty()
    );
    assert_eq!(
        prefs.sidebar_navigation.entries(Section::Footer).items(),
        [item("usage", true)]
    );
}

#[test]
fn usage_fetch_policy_reads_the_same_opt_in_choice_as_the_footer() {
    let prefs = Preferences::default();
    assert_eq!(
        prefs.sidebar_navigation.visibility(Builtin::Usage),
        Visibility::Hidden
    );
    let change = PreferenceChange::SidebarNavigation {
        section: Section::Footer,
        entries: entries(vec![item("usage", true), item("usage", false)]),
    };
    let next = prefs.changed(&change).unwrap();
    assert_eq!(
        next.sidebar_navigation.visibility(Builtin::Usage),
        Visibility::Shown
    );
    assert_eq!(
        next.sidebar_navigation.visibility(Builtin::Search),
        Visibility::Shown
    );
}
