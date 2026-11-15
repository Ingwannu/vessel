//! The language row's choices share the embedded registry with resolution.
//!
//! [Decision Log]
//! - Purpose: a contributor adds a folder without adding a Rust chooser arm.
//! - Constraint: the pseudo locale is only a checking override; unknown stored
//!   tags must not make an otherwise readable preferences file unreadable.
//! - Choice: use the registry's validated `LanguagePreference`, System first,
//!   and the catalog's own short label; ordinary languages name themselves.
//! - Cost: a removed or unknown language follows System on the next launch.

use crate::localization::{LANGUAGES, LanguagePreference, Translator};

impl LanguagePreference {
    /// The chooser's order. Pseudo is deliberately absent from LANGUAGES.
    pub fn choices() -> impl Iterator<Item = Self> {
        std::iter::once(Self::System)
            .chain(LANGUAGES.iter().map(|locale| Self::from_tag(locale.tag())))
    }

    /// System is translated; each language always uses its own name.
    #[must_use]
    pub fn label(self, words: &Translator) -> &str {
        match self {
            Self::System => words.meta().system(),
            Self::Language(_) => {
                crate::localization::resolve_language(self, None, None).native_name()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, Locale};
    use crate::prefs::{PreferenceChange, Preferences, SendKey, Theme};

    #[test]
    fn choices_have_system_first_then_every_ordinary_native_name() {
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        let choices: Vec<_> = LanguagePreference::choices().collect();
        assert_eq!(choices.first(), Some(&LanguagePreference::System));
        assert_eq!(choices.len(), LANGUAGES.len() + 1);
        assert_eq!(choices[0].label(en), "System");
        assert_eq!(choices[0].label(ko), "시스템");
        for (choice, locale) in choices.iter().skip(1).zip(LANGUAGES) {
            assert_eq!(choice.tag(), locale.tag());
            assert_eq!(choice.label(en), locale.native_name());
            assert_eq!(choice.label(ko), locale.native_name());
            assert_ne!(choice.tag(), "en-XA");
        }
    }

    #[test]
    fn stored_language_round_trips_without_changing_other_preferences() {
        let old: Preferences = serde_json::from_str(
            r#"{"appearance":{"theme":"dark"},"composer":{"send":"mod_enter"}}"#,
        )
        .unwrap();
        assert_eq!(old.language, LanguagePreference::System);
        let language = LanguagePreference::from_tag("ko");
        let changed = old.changed(&PreferenceChange::Language(language)).unwrap();
        assert_eq!(changed.language, language);
        assert_eq!(changed.appearance.theme, Theme::Dark);
        assert_eq!(changed.composer.send, SendKey::ModEnter);
        assert_eq!(old.language, LanguagePreference::System);
        let stored = serde_json::to_string(&changed).unwrap();
        assert_eq!(
            serde_json::from_str::<Preferences>(&stored).unwrap(),
            changed
        );
    }

    #[test]
    fn unknown_removed_and_checking_tags_keep_the_rest_of_the_file() {
        for language in ["missing-ZZ", "en-XA", "", "system"] {
            let stored = serde_json::json!({
                "language": language,
                "appearance": {"theme": "dark"},
                "composer": {"send": "mod_enter"},
            });
            let prefs: Preferences = serde_json::from_value(stored).unwrap();
            assert_eq!(prefs.language, LanguagePreference::System);
            assert_eq!(prefs.appearance.theme, Theme::Dark);
            assert_eq!(prefs.composer.send, SendKey::ModEnter);
        }
        for malformed in [
            serde_json::json!(42),
            serde_json::json!(null),
            serde_json::json!({}),
        ] {
            let prefs: Preferences = serde_json::from_value(serde_json::json!({
                "language": malformed, "appearance": {"theme": "dark"},
            }))
            .unwrap();
            assert_eq!(prefs.language, LanguagePreference::System);
            assert_eq!(prefs.appearance.theme, Theme::Dark);
        }
    }
}
