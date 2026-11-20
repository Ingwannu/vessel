//! Text a browser tool carries. Constructors reject an empty value where
//! the tool requires one. The text itself is never put in an error.

use std::fmt;
use std::str::FromStr;

use super::limits::{TextCap, cap_units};
use super::InvalidInput;

/// Text that may be empty: a fill value, or text typed into the focused
/// element. It is kept in the session log as the tool call's input.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct BrowserText(String);

impl BrowserText {
    /// `text`, including an empty string.
    #[must_use]
    pub fn any(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BrowserText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for BrowserText {
    type Err = InvalidInput;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self(text.to_owned()))
    }
}

crate::key::serde_via_text!(BrowserText);

/// Text that must contain at least one character: a key, a function, a
/// wait string, an upload path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct RequiredText(String);

impl RequiredText {
    /// `text` when it is not empty.
    ///
    /// # Errors
    /// [`InvalidInput`] is the caller's reason (`KeyEmpty`,
    /// `FunctionEmpty`, …). An empty string returns that reason.
    pub fn non_empty(text: &str, empty: InvalidInput) -> Result<Self, InvalidInput> {
        if text.is_empty() {
            Err(empty)
        } else {
            Ok(Self(text.to_owned()))
        }
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the text is empty. A constructed value is not.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many bytes the text is.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for RequiredText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for RequiredText {
    type Err = InvalidInput;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::non_empty(text, InvalidInput::EmptyText)
    }
}

crate::key::serde_via_text!(RequiredText);

/// A snapshot's text, cut at 80_000 bytes. Paseo's evaluate preview is
/// cut at 79_000 before its cap marker; this slice uses one cap.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct SnapshotText(String);

impl SnapshotText {
    /// `text`, cut on a character boundary when it is past the cap.
    #[must_use]
    pub fn capped(text: &str) -> (Self, TextCap) {
        let (kept, cap) = cap_units(text, super::limits::MAX_TEXT_BYTES);
        (Self(kept), cap)
    }

    /// The text that fits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SnapshotText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for SnapshotText {
    type Err = InvalidInput;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (text, cap) = Self::capped(text);
        if cap == TextCap::Cut {
            Err(InvalidInput::TextSize)
        } else {
            Ok(text)
        }
    }
}

crate::key::serde_via_text!(SnapshotText);

/// `browser_evaluate`'s result: a string the node does not parse.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct EvaluateJson(String);

impl EvaluateJson {
    /// `json`, cut on a character boundary when it is past the cap.
    /// The node does not parse it.
    #[must_use]
    pub fn capped(json: &str) -> (Self, TextCap) {
        let (kept, cap) = cap_units(json, super::limits::MAX_TEXT_BYTES);
        (Self(kept), cap)
    }

    /// The JSON text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EvaluateJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for EvaluateJson {
    type Err = InvalidInput;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (text, cap) = Self::capped(text);
        if cap == TextCap::Cut {
            Err(InvalidInput::TextSize)
        } else {
            Ok(text)
        }
    }
}

crate::key::serde_via_text!(EvaluateJson);
