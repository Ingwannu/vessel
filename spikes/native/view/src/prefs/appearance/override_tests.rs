//! The real preference edge retains overrides without changing old defaults.
use super::*;
use crate::{
    appearance::{ContentPixels, FontFamily, SyntaxTheme},
    prefs::{PreferenceChange, Preferences},
};

#[test]
fn independent_axes_families_and_syntax_survive_the_stored_document() {
    let initial = Preferences::default();
    let family = FontFamily::parse("'A, B', Pretendard").unwrap();
    let changes = [
        PreferenceChange::ContentSize(ContentSize::try_from(ContentSize::MAX).unwrap()),
        PreferenceChange::InterfaceFamily(family.clone()),
        PreferenceChange::MonoFamily(FontFamily::parse("Geist Mono").unwrap()),
        PreferenceChange::SyntaxTheme(SyntaxTheme::Nord),
    ];
    let mut saved = initial.clone();
    for change in changes {
        saved = saved.changed(&change).unwrap();
    }
    assert_eq!(
        saved.appearance.interface_size,
        initial.appearance.interface_size
    );
    assert_eq!(saved.appearance.code_size, initial.appearance.code_size);
    assert_eq!(saved.appearance.interface_family, family);
    assert_eq!(saved.appearance.content_size.get(), ContentSize::MAX);
    assert_eq!(saved.appearance.syntax_theme, SyntaxTheme::Nord);
    let bytes = serde_json::to_string(&saved).unwrap();
    assert_eq!(serde_json::from_str::<Preferences>(&bytes).unwrap(), saved);
    let changed_palette = saved
        .changed(&PreferenceChange::Theme(Theme::Ghostty))
        .unwrap();
    assert_eq!(
        changed_palette.appearance.interface_family,
        saved.appearance.interface_family
    );
    assert_eq!(
        changed_palette.appearance.content_size,
        saved.appearance.content_size
    );
    assert_eq!(
        changed_palette.appearance.syntax_theme,
        saved.appearance.syntax_theme
    );
}

#[test]
fn malformed_individual_overrides_do_not_erase_valid_siblings() {
    let read: Preferences = serde_json::from_str(r#"{"appearance":{"interface_family":"A;B","mono_family":"Geist Mono","content_size":255,"syntax_theme":"unknown","code_size":13}}"#).unwrap();
    assert!(read.appearance.interface_family.is_default());
    assert_eq!(read.appearance.mono_family.as_str(), "Geist Mono");
    assert_eq!(read.appearance.content_size, ContentSize::default());
    assert_eq!(read.appearance.syntax_theme, SyntaxTheme::One);
    assert_eq!(read.appearance.code_size.get(), 13);
}

#[test]
fn numeric_width_preserves_following_intent_and_legacy_preferences() {
    let follow = ContentWidth::default();
    assert_eq!(follow.commit(ContentPixels::DEFAULT), follow);
    assert!(!follow.resettable());
    let explicit = follow.commit(ContentPixels::try_from(ContentPixels::MAX).unwrap());
    assert!(explicit.resettable());
    assert_eq!(explicit.points(), Some(ContentPixels::MAX));
    let saved = Preferences::default()
        .changed(&PreferenceChange::ContentWidth(explicit))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Preferences>(&serde_json::to_string(&saved).unwrap()).unwrap(),
        saved
    );
    assert_eq!(
        saved
            .changed(&PreferenceChange::ContentWidth(ContentWidth::default()))
            .unwrap()
            .appearance
            .content_width,
        follow
    );
    let legacy: Preferences =
        serde_json::from_str(r#"{"appearance":{"content_width":"full"}}"#).unwrap();
    assert_eq!(legacy.appearance.content_width, ContentWidth::Full);
    let null: Preferences =
        serde_json::from_str(r#"{"appearance":{"content_width":null}}"#).unwrap();
    assert_eq!(null.appearance.content_width, follow);
}
