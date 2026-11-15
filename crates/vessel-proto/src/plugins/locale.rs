//! One existing native resource-language decoder, now shared as pure data.
//! This tag does not select an embedded core Locale or grant translation support.
use crate::key::serde_via_text;
use std::{fmt, str::FromStr};

/// Preserve native's original byte-before-parser resource-name admission.
pub const MAX_LOCALE_TAG_BYTES: usize = 128;

/// Input-free errors preserve the original parser's closed classifications.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum InvalidLocaleTag {
    /// Refuse before parsing or allocating a normalized language identifier.
    #[error("plugin resource language tags exceed {MAX_LOCALE_TAG_BYTES} UTF-8 bytes")]
    Capacity,
    /// The existing language-identifier parser owns grammar and normalization.
    #[error("invalid plugin resource language tag")]
    Syntax(#[source] unic_langid::LanguageIdentifierError),
}

/// Canonical plugin resource tag, independent from core's embedded languages.
/// Unknown but valid identifiers are data, not an installed-language capability.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(type = "string"))]
pub struct LocaleTag(String);

impl FromStr for LocaleTag {
    type Err = InvalidLocaleTag;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.len() > MAX_LOCALE_TAG_BYTES {
            return Err(InvalidLocaleTag::Capacity);
        }
        let language: unic_langid::LanguageIdentifier =
            raw.parse().map_err(InvalidLocaleTag::Syntax)?;
        Ok(Self(language.to_string()))
    }
}
impl LocaleTag {
    /// Canonical spelling used by the resource owner for duplicate detection.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for LocaleTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
serde_via_text!(LocaleTag, "a bounded plugin resource language tag");

#[cfg(test)]
mod tests;
