//! A model's title has Paseo's schema bound, separate from the existing
//! user-title bound. Checking before trimming preserves the model's
//! output contract and cannot silently repair an overlong answer.

use super::{InvalidTitle, SessionTitle};
use std::{fmt, str::FromStr};

/// The raw metadata title bound, shared by parsing and model instructions.
pub const MAX_GENERATED_TITLE_UNITS: usize = 80;

/// A title validated against the metadata schema before it reaches an
/// actor. UTF-16 units reproduce the reference schema's string length;
/// ordinary `SessionTitle` keeps its established Unicode-scalar bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedTitle(SessionTitle);

/// A schema refusal retains the ordinary title validator's source.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidGeneratedTitle {
    /// Count the raw model text before its padding is trimmed.
    #[error("generated title exceeds {MAX_GENERATED_TITLE_UNITS} UTF-16 units")]
    TooLong,
    /// Blank, control characters, or another ordinary title invariant.
    #[error("generated text is not a valid title: {0}")]
    Title(#[source] InvalidTitle),
}

impl GeneratedTitle {
    /// Hand the checked value to the ordinary title/event representation.
    #[must_use]
    pub fn into_title(self) -> SessionTitle {
        self.0
    }
}

impl FromStr for GeneratedTitle {
    type Err = InvalidGeneratedTitle;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.encode_utf16().count() > MAX_GENERATED_TITLE_UNITS {
            return Err(InvalidGeneratedTitle::TooLong);
        }
        text.parse().map(Self).map_err(InvalidGeneratedTitle::Title)
    }
}

impl fmt::Display for GeneratedTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

crate::key::serde_via_text!(GeneratedTitle);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_length_is_raw_utf16_and_never_repaired_by_trimming() {
        assert!(
            "a".repeat(MAX_GENERATED_TITLE_UNITS)
                .parse::<GeneratedTitle>()
                .is_ok()
        );
        let padded = format!(" {} ", "a".repeat(MAX_GENERATED_TITLE_UNITS));
        let error = padded.parse::<GeneratedTitle>().unwrap_err();
        assert_eq!(error, InvalidGeneratedTitle::TooLong);
        assert_eq!(
            error.to_string(),
            format!("generated title exceeds {MAX_GENERATED_TITLE_UNITS} UTF-16 units")
        );
        let astral_count = MAX_GENERATED_TITLE_UNITS
            .checked_div("😀".encode_utf16().count())
            .unwrap();
        assert!("😀".repeat(astral_count).parse::<GeneratedTitle>().is_ok());
        assert!(matches!(
            "😀".repeat(astral_count + 1).parse::<GeneratedTitle>(),
            Err(InvalidGeneratedTitle::TooLong)
        ));
        assert_eq!(
            "  Parse identifiers  "
                .parse::<GeneratedTitle>()
                .unwrap()
                .into_title()
                .as_str(),
            "Parse identifiers"
        );
    }

    #[test]
    fn title_rules_are_preserved_and_wire_decoding_checks_the_same_bound() {
        for text in ["", "   ", "line\nbreak"] {
            assert!(matches!(
                text.parse::<GeneratedTitle>(),
                Err(InvalidGeneratedTitle::Title(_))
            ));
        }
        let padded = format!("\" {} \"", "a".repeat(MAX_GENERATED_TITLE_UNITS));
        assert!(serde_json::from_str::<GeneratedTitle>(&padded).is_err());
        let title: GeneratedTitle = serde_json::from_str("\"Keep identifiers\"").unwrap();
        assert_eq!(title.into_title().as_str(), "Keep identifiers");
    }
}
