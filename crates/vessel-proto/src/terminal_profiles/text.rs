//! Typed profile fields share one validation boundary and redact their Debug.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::ProfileError;

/// Identifies a field's grammar without echoing its potentially sensitive text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    /// Stable row identity, independent from display name.
    Id,
    /// One-line user-facing label.
    Name,
    /// Literal executable or path template.
    Program,
    /// One argv element, preserving whitespace.
    Argument,
    /// User prompt to substitute literally.
    Prompt,
    /// A provider/icon catalog key.
    Icon,
}

impl TextKind {
    fn validate(self, value: &str, max: usize) -> Result<(), ProfileError> {
        if value.len() > max {
            return Err(ProfileError::TooLong { kind: self, max });
        }
        if value.contains('\0') {
            return Err(ProfileError::Character(self));
        }
        match self {
            Self::Id | Self::Icon => {
                if value.is_empty() {
                    return Err(ProfileError::Empty(self));
                }
                if !value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
                {
                    return Err(ProfileError::Character(self));
                }
                if self == Self::Id && value == "blank" {
                    return Err(ProfileError::ReservedId);
                }
            }
            Self::Name => {
                if value.trim().is_empty() {
                    return Err(ProfileError::Empty(self));
                }
                if value.chars().any(char::is_control) {
                    return Err(ProfileError::Character(self));
                }
            }
            Self::Program => {
                if value.trim().is_empty() {
                    return Err(ProfileError::Empty(self));
                }
            }
            Self::Argument | Self::Prompt => {}
        }
        Ok(())
    }
}

// Every field has a distinct type, while serde and validation cannot drift.
macro_rules! profile_text {
    ($name:ident, $kind:ident, $max:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(String);

        impl $name {
            /// Maximum UTF-8 bytes accepted at construction and decode.
            pub const MAX_BYTES: usize = $max;

            /// Validate at input, without trimming meaningful argv text.
            /// # Errors
            /// Rejects the field's invalid characters or size without echoing it.
            pub fn parse(value: impl Into<String>) -> Result<Self, ProfileError> {
                let value = value.into();
                TextKind::$kind.validate(&value, Self::MAX_BYTES)?;
                Ok(Self(value))
            }

            /// Borrow the original validated text at the display/launch boundary.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = ProfileError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
    };
}

profile_text!(Id, Id, 64, "A stable terminal profile row identity.");
profile_text!(
    Name,
    Name,
    800,
    "A one-line profile label, allowing long Unicode names."
);
profile_text!(
    Program,
    Program,
    4096,
    "A literal executable/path template, never a shell command."
);
profile_text!(
    Argument,
    Argument,
    16 * 1024,
    "One literal argv template/result, with whitespace preserved."
);
profile_text!(
    Prompt,
    Prompt,
    64 * 1024,
    "A bounded prompt to substitute as literal argv text."
);
profile_text!(
    Icon,
    Icon,
    Id::MAX_BYTES,
    "An optional catalog icon identity, independent of the program."
);
