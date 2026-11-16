//! The default/explicit/pending cases from Paseo's hosts/appearance.test.ts.

use super::*;

#[test]
fn untouched_local_hides_and_remote_names_after_lookup() {
    let appearance = Appearance::default();
    assert_eq!(appearance.color, Color::None);
    assert_eq!(appearance.badge(Locality::Local), Badge::Hidden);
    assert_eq!(
        appearance.badge(Locality::Remote),
        Badge::Visible(BadgeContent::Name)
    );
}

#[test]
fn only_the_automatic_choice_waits_for_local_detection() {
    assert_eq!(
        Appearance::default().badge(Locality::Pending),
        Badge::Pending
    );
    for (choice, expected) in [
        (BadgePreference::Name, Badge::Visible(BadgeContent::Name)),
        (BadgePreference::Icon, Badge::Visible(BadgeContent::Icon)),
        (BadgePreference::Hidden, Badge::Hidden),
    ] {
        let appearance = Appearance {
            color: Color::Teal,
            badge: choice,
        };
        for locality in [Locality::Pending, Locality::Local, Locality::Remote] {
            assert_eq!(appearance.badge(locality), expected);
        }
    }
}

#[test]
fn locality_refresh_never_replaces_an_explicit_choice() {
    let appearance = Appearance {
        color: Color::Violet,
        badge: BadgePreference::Icon,
    };
    for locality in [Locality::Remote, Locality::Pending, Locality::Local] {
        assert_eq!(
            appearance.badge(locality),
            Badge::Visible(BadgeContent::Icon)
        );
    }
    assert_eq!(appearance.color, Color::Violet);
    assert_eq!(appearance.badge, BadgePreference::Icon);
}

#[test]
fn choosing_automatic_again_restores_the_contextual_default() {
    let mut appearance = Appearance {
        color: Color::Blue,
        badge: BadgePreference::Name,
    };
    assert_eq!(
        appearance.badge(Locality::Local),
        Badge::Visible(BadgeContent::Name)
    );
    appearance.badge = BadgePreference::Automatic;
    assert_eq!(appearance.badge(Locality::Pending), Badge::Pending);
    assert_eq!(appearance.badge(Locality::Local), Badge::Hidden);
    assert_eq!(
        appearance.badge(Locality::Remote),
        Badge::Visible(BadgeContent::Name)
    );
    assert_eq!(appearance.color, Color::Blue);
}
