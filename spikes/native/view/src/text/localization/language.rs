//! Locale selection is pure and shared by every host. Files are discovered
//! at build time; no new enum arm is needed to add a translation folder.

use super::{ENGLISH, LANGUAGES, PSEUDO};

/// One embedded language, identified by its canonical tag.
/// Its index is private so a caller cannot construct a nonexistent locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locale {
    pub(super) index: usize,
    pub(super) tag: &'static str,
    pub(super) native_name: &'static str,
    pub(super) complete: bool,
}

impl Locale {
    /// Canonical BCP 47 tag, suitable for storing a language preference.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        self.tag
    }

    /// The language's own name, supplied by its folder's `meta-name`.
    #[must_use]
    pub const fn native_name(self) -> &'static str {
        self.native_name
    }

    /// Whether every currently migrated English message is translated.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        self.complete
    }

    /// An exact embedded, ordinary language; the pseudo-language is private
    /// to the checking override and cannot become a stored preference.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        LANGUAGES
            .iter()
            .copied()
            .find(|locale| locale.tag.eq_ignore_ascii_case(tag))
    }
}

/// A person's language choice. UI persistence uses the same type in slice 2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LanguagePreference {
    /// Follow the operating system (or the browser's primary language).
    #[default]
    System,
    /// An ordinary embedded language, never the pseudo-language.
    Language(LanguageTag),
}

/// A stored language tag that this build embeds. Its private constructor
/// prevents a checking-only pseudo-language becoming a person's preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageTag(Locale);

impl LanguagePreference {
    /// Read a stored choice. Unknown tags become System, keeping the rest of
    /// the preferences document readable after a language is removed.
    #[must_use]
    pub fn from_tag(tag: &str) -> Self {
        Locale::from_tag(tag).map_or(Self::System, |locale| Self::Language(LanguageTag(locale)))
    }

    /// Canonical stored spelling; the registry is the only source of tags.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Language(LanguageTag(locale)) => locale.tag,
        }
    }
}

impl serde::Serialize for LanguagePreference {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.tag())
    }
}

impl<'de> serde::Deserialize<'de> for LanguagePreference {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let tag = <String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::from_tag(&tag))
    }
}

/// Choose an override, a preference, or the system's primary tag, in order.
/// A tag tries its exact form, then its base language, then English.
/// The pseudo-language can only be selected through `override_tag`.
#[must_use]
pub fn resolve_language(
    preference: LanguagePreference,
    system_tag: Option<&str>,
    override_tag: Option<&str>,
) -> Locale {
    resolve_languages(preference, system_tag, override_tag)
}

/// Choose from the system's preferred tags in order, after the checking
/// override and explicit preference. Unsupported tags do not hide a later
/// supported language; English is used only once the list is exhausted.
///
/// The host supplies its existing locale provider's iterator, so view performs
/// no I/O and browser/native hosts share the same selection rule. As with the
/// single-tag adapter, call this at selection events, never while drawing.
#[must_use]
pub fn resolve_languages(
    preference: LanguagePreference,
    system_tags: impl IntoIterator<Item = impl AsRef<str>>,
    override_tag: Option<&str>,
) -> Locale {
    if let Some(tag) = override_tag {
        if normalize(tag).eq_ignore_ascii_case(PSEUDO.tag) {
            return PSEUDO;
        }
        return match_tag(tag, LANGUAGES).unwrap_or(ENGLISH);
    }
    match preference {
        LanguagePreference::System => system_tags
            .into_iter()
            .find_map(|tag| match_tag(tag.as_ref(), LANGUAGES))
            .unwrap_or(ENGLISH),
        LanguagePreference::Language(LanguageTag(locale)) => locale,
    }
}

fn normalize(tag: &str) -> String {
    tag.split(['.', '@'])
        .next()
        .unwrap_or_default()
        .replace('_', "-")
}

fn match_tag(tag: &str, languages: &[Locale]) -> Option<Locale> {
    let tag = normalize(tag);
    let find = |tag: &str| {
        languages
            .iter()
            .copied()
            .find(|locale| locale.tag.eq_ignore_ascii_case(tag))
    };
    find(&tag).or_else(|| tag.split('-').next().and_then(find))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_region_wins_before_base_language() {
        let portuguese = Locale {
            index: 0,
            tag: "pt",
            native_name: "Português",
            complete: false,
        };
        let brazilian = Locale {
            index: 1,
            tag: "pt-BR",
            native_name: "Português brasileiro",
            complete: false,
        };
        let languages = [portuguese, brazilian];
        assert_eq!(match_tag("pt-BR", &languages), Some(brazilian));
        assert_eq!(match_tag("pt_PT.UTF-8", &languages), Some(portuguese));
        assert_eq!(match_tag("pt-br", &languages), Some(brazilian));
        assert_eq!(match_tag("de-DE", &languages), None);
    }
}

#[cfg(test)]
mod preferred_tests {
    use super::*;

    #[test]
    fn system_uses_the_first_supported_tag_not_just_the_first_tag() {
        let cases: &[(&[&str], &str)] = &[
            (&[], "en"),
            (&["zz-ZZ", "ko-KR", "en-US"], "ko"),
            (&["zz-ZZ", "en-US", "ko-KR"], "en"),
            (&["ko-KR", "en-US"], "ko"),
            (&["en-US", "ko-KR"], "en"),
            (&["zz-ZZ", "KO_kr.UTF-8"], "ko"),
            (&["zz-ZZ", "missing"], "en"),
        ];
        for (tags, expected) in cases {
            assert_eq!(
                resolve_languages(LanguagePreference::System, tags.iter(), None).tag(),
                *expected,
                "system tags {tags:?}"
            );
        }
        // The host provider yields owned strings; neither a collected list nor
        // an additional parser is needed at the app/view boundary.
        let host_tags = [String::from("zz-ZZ"), String::from("ko-KR")];
        assert_eq!(
            resolve_languages(LanguagePreference::System, host_tags, None).tag(),
            "ko"
        );
    }

    #[test]
    fn override_and_preference_keep_priority_over_the_ordered_system_list() {
        let ko = Locale::from_tag("ko").unwrap();
        let cases = [
            (LanguagePreference::from_tag("ko"), None, ko),
            (LanguagePreference::from_tag("ko"), Some("en"), ENGLISH),
            (LanguagePreference::from_tag("ko"), Some("unknown"), ENGLISH),
            (LanguagePreference::from_tag("en"), Some("en-XA"), PSEUDO),
            (LanguagePreference::from_tag("removed"), None, ko),
        ];
        for (preference, checking, expected) in cases {
            assert_eq!(
                resolve_languages(preference, ["zz-ZZ", "ko-KR"], checking),
                expected
            );
        }
    }
}
