//! Theme-cycle contracts through the existing registry and preferences.

use super::*;
use crate::prefs::Theme;
use crate::shortcuts::ShortcutScope;

#[test]
fn theme_cycle_uses_the_primary_modifier_and_alt_on_both_platforms() {
    let none = Overrides::new();
    let expected = run(Action::CycleTheme);
    assert_eq!(matched(&chord("Alt+Meta+T"), None, &none, 0, MAC), expected);
    assert_eq!(
        matched(&chord("Ctrl+Alt+T"), None, &none, 0, OTHER),
        expected
    );
    for platform in [MAC, OTHER] {
        assert_eq!(
            matched(&chord("Alt+T"), None, &none, 0, platform),
            Matched::None
        );
        assert_eq!(
            matched(&chord("Mod+Shift+T"), None, &none, 0, platform),
            Matched::None
        );
    }
    let option = KeyPress {
        meta: true,
        alt: true,
        ..press("†", "KeyT")
    };
    assert_eq!(
        matched(&chord_of(&option, MAC).unwrap(), None, &none, 0, MAC),
        expected
    );
}

#[test]
fn theme_cycle_is_excluded_in_the_command_center_on_both_platforms() {
    for platform in [MAC, OTHER] {
        assert!(!Action::CycleTheme.enabled(ShortcutScope::CommandCenter, platform));
        assert!(Action::CycleTheme.enabled(ShortcutScope::Elsewhere, platform));
    }
}

#[test]
fn terminal_focus_keeps_theme_cycle_off_mac_but_allows_it_on_mac() {
    assert!(!Action::CycleTheme.enabled(ShortcutScope::Terminal, OTHER));
    assert!(Action::CycleTheme.enabled(ShortcutScope::Terminal, MAC));
}

#[test]
fn rebound_theme_cycle_keeps_its_scope_and_appears_in_general() {
    let changed = overrides(&[("cycle-theme", &["Mod+J"])]);
    let found = matched(&chord("Mod+J"), None, &changed, 0, OTHER);
    assert_eq!(found, run(Action::CycleTheme));
    assert_eq!(
        matched(&chord("Mod+Alt+T"), None, &changed, 0, OTHER),
        Matched::None
    );
    let Matched::Run { action, .. } = found else {
        panic!("the rebound action must be reachable");
    };
    assert!(!action.enabled(ShortcutScope::Terminal, OTHER));
    assert!(!action.enabled(ShortcutScope::CommandCenter, OTHER));
    assert_eq!(action.group(), ShortcutGroup::General);
    let row = shortcut_row(action, &changed, OTHER, Lang::En.shortcuts());
    assert_eq!(row.words, ["Ctrl+J"]);
    assert_eq!(Lang::En.shortcuts().label(action), "Cycle theme");
    assert_eq!(Lang::Ko.shortcuts().label(action), "테마 전환");
    assert_eq!(
        clash_of(Action::Find, &binding("Mod+J"), &changed, OTHER),
        Some(Clash::Taken(Action::CycleTheme))
    );
}

#[test]
fn a_cycle_uses_the_preference_order_not_the_system_first_picker_order() {
    let mut prefs = Preferences::default()
        .changed(&PreferenceChange::Theme(Theme::Light))
        .unwrap();
    for expected in [
        Theme::Dark,
        Theme::System,
        Theme::Zinc,
        Theme::Midnight,
        Theme::Claude,
        Theme::Ghostty,
        Theme::PureBlack,
        Theme::Light,
    ] {
        let next = prefs.appearance.theme.next();
        prefs = prefs.changed(&PreferenceChange::Theme(next)).unwrap();
        assert_eq!(prefs.appearance.theme, expected);
    }
}
