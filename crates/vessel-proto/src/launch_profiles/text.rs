//! Validating text and distinct appearance-key domains; no input in Debug.

use std::{fmt, str::FromStr, sync::Arc};

use super::InvalidLaunchValue;

macro_rules! text {
    ($name:ident, $doc:literal, $bound:ident, $size:expr, $error:ident, $check:expr) => {
        #[doc = $doc]
        #[derive(Clone, PartialEq, Eq, Hash, serde::Serialize)]
        #[serde(transparent)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Arc<str>);
        impl $name {
            /// The single bound used by parsing, errors and boundary fixtures.
            pub const $bound: usize = $size;
            /// Canonical text. Reading it is an explicit public-data operation.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl FromStr for $name {
            type Err = InvalidLaunchValue;
            fn from_str(input: &str) -> Result<Self, Self::Err> {
                let value = input.trim();
                let valid: fn(&str) -> bool = $check;
                if valid(value) {
                    Ok(Self(Arc::from(value)))
                } else {
                    Err(InvalidLaunchValue::$error)
                }
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(..)"))
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
        // The general text helper preserves serde's diagnostics, which can
        // echo an unexpected scalar. This public-guidance boundary does not.
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
                let text: std::borrow::Cow<'de, str> = super::codec::decode(input)?;
                text.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

text!(
    LaunchProfileName,
    "A canonical, single-line launch profile name.",
    MAX_CHARS,
    80,
    Name,
    |s| !s.is_empty()
        && s.chars().count() <= LaunchProfileName::MAX_CHARS
        && !s
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
);
text!(
    LaunchProfileNotes,
    "Trimmed public guidance, intentionally readable by Read agents.",
    MAX_BYTES,
    4_096,
    Notes,
    |s| !s.is_empty()
        && s.len() <= LaunchProfileNotes::MAX_BYTES
        && !s
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
);
text!(
    LaunchAppearanceKey,
    "A bounded registry key shared by distinct icon/color domains.",
    MAX_BYTES,
    64,
    Appearance,
    |s| !s.is_empty()
        && s.len() <= LaunchAppearanceKey::MAX_BYTES
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
);

macro_rules! appearance {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        #[serde(transparent)]
        pub struct $name(LaunchAppearanceKey);
        impl FromStr for $name {
            type Err = InvalidLaunchValue;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse().map(Self)
            }
        }
        impl $name {
            /// The registry key; unknown keys are a client's fallback, not glyphs.
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}
appearance!(
    LaunchIconKey,
    "An icon-registry key, never interchangeable with a color key."
);
appearance!(
    LaunchColorKey,
    "A color-registry key, never interchangeable with an icon key."
);
