//! Provider identifiers are parsed once; rejected input is never echoed.

use std::{fmt, str::FromStr, sync::Arc};

/// Maximum provider identifier bytes; a local resource bound above Paseo's grammar.
pub const MAX_PROVIDER_ID: usize = 128;
/// Mode IDs include ACP's URI identifiers, so this bound admits complete URIs.
pub const MAX_MODE_ID: usize = 1024;

/// A provider value violates its grammar or resource bound.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidProvider {
    /// Provider IDs follow Paseo's lower-case identifier grammar.
    #[error(
        "provider id must start with a-z, contain only a-z, digits or -, and fit {MAX_PROVIDER_ID} bytes"
    )]
    Id,
    /// Native mode identifiers are opaque but cannot carry controls.
    #[error("mode id must be nonempty, without control characters, and fit {MAX_MODE_ID} bytes")]
    Mode,
    /// A catalogue cannot select a mode it does not contain.
    #[error("default mode is absent from the mode catalogue")]
    MissingDefault,
    /// Duplicated native identifiers would make a selection ambiguous.
    #[error("mode catalogue repeats an identifier")]
    DuplicateMode,
    /// A list cannot grow without a bound at the decoding edge.
    #[error("mode catalogue exceeds {} entries", super::ModeCatalogue::MAX)]
    TooManyModes,
}

macro_rules! identifier {
    ($(#[$doc:meta])* $name:ident, $error:ident, $valid:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        pub struct $name(Arc<str>);

        impl $name {
            /// Native text, retained verbatim for selection and identity.
            #[must_use]
            pub fn as_str(&self) -> &str { &self.0 }
        }

        impl FromStr for $name {
            type Err = InvalidProvider;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let valid: fn(&str) -> bool = $valid;
                if valid(text) { Ok(Self(Arc::from(text))) }
                else { Err(InvalidProvider::$error) }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
        }

        crate::key::serde_via_text!($name);
    };
}

identifier! {
    /// An open provider identity, distinct from a harness or a model route.
    ProviderId, Id, |text| {
        text.len() <= MAX_PROVIDER_ID
            && text.bytes().next().is_some_and(|first| first.is_ascii_lowercase())
            && text.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }
}

identifier! {
    /// Native provider permission-mode identity, including ACP URI modes.
    ModeId, Mode, |text| {
        !text.is_empty() && text.len() <= MAX_MODE_ID && !text.chars().any(char::is_control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_ids_have_paseos_grammar_and_bounded_decoding() {
        for id in ["claude", "custom-2", "omp"] {
            assert_eq!(id.parse::<ProviderId>().unwrap().as_str(), id);
        }
        for id in ["", "1bad", "Mixed", "under_score", "space here", "bad\n"] {
            assert!(id.parse::<ProviderId>().is_err());
        }
        assert!("a".repeat(MAX_PROVIDER_ID).parse::<ProviderId>().is_ok());
        assert!(
            "a".repeat(MAX_PROVIDER_ID + 1)
                .parse::<ProviderId>()
                .is_err()
        );
        assert!(serde_json::from_str::<ProviderId>(r#""BAD""#).is_err());
    }

    #[test]
    fn mode_ids_retain_uris_and_refuse_controls_or_unbounded_text() {
        let uri = "https://agentclientprotocol.com/protocol/session-modes#agent";
        let mode = uri.parse::<ModeId>().unwrap();
        assert_eq!(
            serde_json::from_str::<ModeId>(&serde_json::to_string(&mode).unwrap()).unwrap(),
            mode
        );
        assert!("x".repeat(MAX_MODE_ID).parse::<ModeId>().is_ok());
        assert!("x".repeat(MAX_MODE_ID + 1).parse::<ModeId>().is_err());
        for text in ["", "bad\0", "bad\u{85}"] {
            assert!(text.parse::<ModeId>().is_err());
        }
    }
}
