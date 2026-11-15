//! Theme ids, order and lenient file compatibility without any UI dependency.

use super::*;
use crate::Lang;
use crate::prefs::{PreferenceChange, Preferences};

#[test]
fn chooser_starts_with_system_but_shortcut_follows_the_source_catalogue() {
    // Paseo ThemePicker19-23 versus theme.ts783-832/884-888.
    assert_eq!(
        Theme::ALL,
        [
            Theme::System,
            Theme::Light,
            Theme::Dark,
            Theme::Zinc,
            Theme::Midnight,
            Theme::Claude,
            Theme::Ghostty,
            Theme::PureBlack
        ]
    );
    let cycle = [
        Theme::Light,
        Theme::Dark,
        Theme::System,
        Theme::Zinc,
        Theme::Midnight,
        Theme::Claude,
        Theme::Ghostty,
        Theme::PureBlack,
    ];
    for (current, expected) in cycle.into_iter().zip(cycle.into_iter().cycle().skip(1)) {
        assert_eq!(current.next(), expected);
    }
    assert_eq!(Theme::default(), Theme::System);
}

#[test]
fn every_choice_round_trips_through_the_existing_change_and_preference_document() {
    let ids = [
        "system",
        "light",
        "dark",
        "zinc",
        "midnight",
        "claude",
        "ghostty",
        "pure_black",
    ];
    for (theme, id) in Theme::ALL.into_iter().zip(ids) {
        assert_eq!(serde_json::to_string(&theme).unwrap(), format!("\"{id}\""));
        let change: PreferenceChange =
            serde_json::from_str(&format!(r#"{{"set":"theme","value":"{id}"}}"#)).unwrap();
        let changed = Preferences::default().changed(&change).unwrap();
        assert_eq!(changed.appearance.theme, theme);
        let saved = serde_json::to_string(&changed).unwrap();
        assert_eq!(
            serde_json::from_str::<Preferences>(&saved).unwrap(),
            changed
        );
    }
}

#[test]
fn unknown_plugin_or_malformed_theme_defaults_only_that_field() {
    for unknown in ["plugin", "sepia", "Zinc", "pureBlack"] {
        let file = format!(r#"{{"appearance":{{"theme":"{unknown}","code_size":13}}}}"#);
        let read: Preferences = serde_json::from_str(&file).unwrap();
        assert_eq!(read.appearance.theme, Theme::System);
        assert_eq!(read.appearance.code_size.get(), 13);
        assert!(
            serde_json::from_str::<PreferenceChange>(&format!(
                r#"{{"set":"theme","value":"{unknown}"}}"#
            ))
            .is_err()
        );
    }
}

#[test]
fn names_follow_the_source_with_the_shipped_korean_light_and_dark_preserved() {
    let en = [
        "System",
        "Light",
        "Dark",
        "Zinc",
        "Midnight",
        "Claude",
        "Ghostty",
        "Pure black",
    ];
    let ko = [
        "시스템",
        "밝게",
        "어둡게",
        "Zinc",
        "Midnight",
        "Claude",
        "Ghostty",
        "순수 검정",
    ];
    for (lang, names) in [(Lang::En, en), (Lang::Ko, ko)] {
        assert_eq!(Theme::ALL.map(|theme| lang.settings().theme(theme)), names);
    }
}
