//! A per-device display label, never a book alias or authenticated identity.
//!
//! Paseo trims the label with JavaScript String.trim and rejects blank input.
//! Unicode labels remain legal; the machine book's ASCII grammar is unrelated.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use vessel_proto::NodeId;

/// A nonblank display label, with source-compatible outer whitespace removed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Label(String);

/// A label with no visible value; its error never repeats the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a machine display label must not be blank")]
pub struct EmptyLabel;

impl Label {
    /// Parse user-facing text without imposing the CLI book's name grammar.
    ///
    /// # Errors
    /// The label is empty after the source's outer whitespace is removed.
    pub fn parse(text: &str) -> Result<Self, EmptyLabel> {
        normalized(text).map(|text| Self(text.to_owned()))
    }

    /// The label as shown; no normalization remains for a screen to perform.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A missing/blank legacy label falls back to the machine identity, as
    /// Paseo's badge selector does. A typed `NodeId` always prints a nonempty id.
    #[must_use]
    pub fn or_node(label: Option<&Self>, node: NodeId) -> Self {
        label.cloned().unwrap_or_else(|| Self(node.to_string()))
    }
}

impl TryFrom<String> for Label {
    type Error = EmptyLabel;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let trimmed = normalized(&text)?;
        if trimmed.len() == text.len() {
            Ok(Self(text))
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }
}

impl FromStr for Label {
    type Err = EmptyLabel;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl From<Label> for String {
    fn from(label: Label) -> Self {
        label.0
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn normalized(text: &str) -> Result<&str, EmptyLabel> {
    // JS trims BOM and does not trim NEL, unlike Rust's str::trim. Match the
    // source's WhiteSpace + LineTerminator set once at the input boundary.
    let text = text.trim_matches(|c| {
        matches!(c,
            '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
            '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
            '\u{205f}' | '\u{3000}' | '\u{feff}'
        )
    });
    if text.is_empty() {
        Err(EmptyLabel)
    } else {
        Ok(text)
    }
}

#[cfg(test)]
mod tests;
