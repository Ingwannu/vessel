//! Validated font-stack preference. No font loading or UI dependency.
//!
//! [Decision Log]
//! - Purpose: the Appearance family draft must not apply malformed text.
//! - Reference: Paseo hooks/use-settings/storage.ts sanitizeFontFamily.
//! - Choice: validate at the input/serde edge; retain its trimmed spelling.
//!   A UTF-16 count matches JavaScript's length rather than a UTF-8 byte cap.
//! - Cost: native font-family resolution is a later kit concern, not CSS here.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
mod faces;
pub use faces::{FaceName, FamilyFace, GenericFamily};

/// Longest family preference, measured as upstream JavaScript UTF-16 units.
pub const MAX_FAMILY_UNITS: usize = 200;

/// A safe, trimmed font-family preference; empty means the default stack.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FontFamily(Arc<FamilyData>);

// One immutable parsed value travels through saved snapshots and command
// queues. Sharing keeps those snapshots cheap and prevents family support
// from inflating every variant of the existing command/event enums.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FamilyData {
    spelling: String,
    faces: Vec<FamilyFace>,
}

/// Why a family draft cannot be committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FamilyError {
    /// Its UTF-16 length exceeds the source limit.
    #[error("font family exceeds {MAX_FAMILY_UNITS} UTF-16 units")]
    TooLong,
    /// A declaration or control character can corrupt a shared style.
    #[error("font family contains a forbidden character")]
    Forbidden,
    /// Quotes, escapes or unquoted CSS constructs are unfinished/unsupported.
    #[error("font family has an invalid quoted or escaped construct")]
    Construct,
}

impl FontFamily {
    /// Validate the entire draft before it can become a preference.
    ///
    /// # Errors
    /// A length, declaration/control character or quoting violation.
    pub fn parse(text: &str) -> Result<Self, FamilyError> {
        let text = super::input::trim(text);
        if text.encode_utf16().count() > MAX_FAMILY_UNITS {
            return Err(FamilyError::TooLong);
        }
        if text
            .chars()
            .any(|ch| ch <= '\u{1f}' || ";{}<>".contains(ch))
        {
            return Err(FamilyError::Forbidden);
        }
        check_constructs(text)?;
        Ok(Self(Arc::new(FamilyData {
            spelling: text.to_owned(),
            faces: faces::decode(text),
        })))
    }

    /// Parsed stack entries; resolution never reparses the saved spelling.
    #[must_use]
    pub fn faces(&self) -> &[FamilyFace] {
        &self.0.faces
    }

    /// Whether the embedded default stack is selected; omit it from old files.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.0.spelling.is_empty()
    }

    /// The retained trimmed value; empty uses the normal default stack.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0.spelling
    }
}

fn check_constructs(text: &str) -> Result<(), FamilyError> {
    let mut quote = None;
    let mut escaped = false;
    for ch in text.chars() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if let Some(current) = quote {
            if ch == current {
                quote = None;
            }
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if "()[]/*".contains(ch) {
            return Err(FamilyError::Construct);
        }
    }
    if quote.is_some() || escaped {
        Err(FamilyError::Construct)
    } else {
        Ok(())
    }
}

impl std::fmt::Display for FontFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.spelling)
    }
}

impl TryFrom<String> for FontFamily {
    type Error = FamilyError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<FontFamily> for String {
    fn from(family: FontFamily) -> Self {
        family.0.spelling.clone()
    }
}

#[cfg(test)]
mod tests;
