//! Validated names, normalized identity and catalog colors.

use std::{fmt, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::key::serde_via_text;

/// Display spelling after ECMAScript whitespace collapse and trim.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct LabelName(Arc<str>);

/// No visible label name remains after normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a workspace label name cannot be empty")]
pub struct LabelNameError;

impl FromStr for LabelName {
    type Err = LabelNameError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let text = raw
            .split(is_reference_whitespace)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            return Err(LabelNameError);
        }
        Ok(Self(text.into()))
    }
}

impl fmt::Display for LabelName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl LabelName {
    /// The host's retained spelling; do not replace this with the folded key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Compare normalized identities without losing case-only display edits.
    #[must_use]
    pub fn key(&self) -> LabelKey {
        LabelKey(self.0.to_lowercase().into())
    }
}

serde_via_text!(LabelName);

/// Catalog identity, derived only from a validated display name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabelKey(Arc<str>);

impl LabelKey {
    /// SQL key binding and deterministic ordering share this one fold.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Trim only the reference's whitespace at the edges. Query matching reuses
/// this predicate without collapsing the query's internal spaces.
#[must_use]
pub fn trim_whitespace(text: &str) -> &str {
    text.trim_matches(is_reference_whitespace)
}

// Rust's Unicode whitespace differs from JavaScript \s at FEFF and 0085.
// Match the reference grammar explicitly so imported identities do not drift.
fn is_reference_whitespace(character: char) -> bool {
    matches!(character, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}'
        | '\u{3000}' | '\u{feff}')
}

/// The reference catalog palette; painting belongs to the native kit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LabelColor {
    /// Default create color.
    #[default]
    Violet,
    /// Sky blue.
    Sky,
    /// Emerald green.
    Emerald,
    /// Orange.
    Orange,
    /// Pink.
    Pink,
    /// Indigo.
    Indigo,
    /// Teal.
    Teal,
    /// Red.
    Red,
    /// Amber.
    Amber,
    /// Blue.
    Blue,
}

impl LabelColor {
    /// Picker swatches retain the protocol's reference order.
    pub const ALL: [Self; 10] = [
        Self::Violet,
        Self::Sky,
        Self::Emerald,
        Self::Orange,
        Self::Pink,
        Self::Indigo,
        Self::Teal,
        Self::Red,
        Self::Amber,
        Self::Blue,
    ];
}

/// A host-owned definition; workspace assignments resolve its current color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Label {
    /// Host spelling.
    pub name: LabelName,
    /// Host color.
    pub color: LabelColor,
}

/// Desired assignment is explicit at every request boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Assignment {
    /// Create an unknown definition if necessary and add its name.
    Assigned,
    /// Remove an assignment; never create an unknown definition.
    Unassigned,
}

/// Advisory or committed affected workspace count; not a deletion lock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct LabelCount(u32);

impl LabelCount {
    /// Checked conversion at the SQL boundary precedes construction.
    #[must_use]
    pub const fn new(count: u32) -> Self {
        Self(count)
    }
    /// Public presentation count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_validation_matches_ecmascript_whitespace_and_preserves_case() {
        assert_eq!(trim_whitespace("\u{feff} Ready\u{feff}"), "Ready");
        assert_eq!(trim_whitespace("\u{0085}Ready"), "\u{0085}Ready");
        let name: LabelName = "\u{feff}  In\tProgress\n\u{feff}".parse().unwrap();
        assert_eq!(name.as_str(), "In Progress");
        assert_eq!(
            name.key(),
            "in progress".parse::<LabelName>().unwrap().key()
        );
        assert_ne!(name, "in progress".parse::<LabelName>().unwrap());
        assert!("\u{feff}\t".parse::<LabelName>().is_err());
        assert_eq!(
            "a\u{0085}b".parse::<LabelName>().unwrap().as_str(),
            "a\u{0085}b"
        );
        let long = "한".repeat(4096);
        assert_eq!(long.parse::<LabelName>().unwrap().as_str(), long);
        assert!(serde_json::from_str::<LabelName>("\"  \"").is_err());
    }

    #[test]
    fn colors_are_closed_and_round_trip_in_reference_order() {
        for color in LabelColor::ALL {
            let encoded = serde_json::to_string(&color).unwrap();
            assert_eq!(serde_json::from_str::<LabelColor>(&encoded).unwrap(), color);
        }
        assert!(serde_json::from_str::<LabelColor>("\"purple\"").is_err());
        assert_eq!(LabelColor::default(), LabelColor::Violet);
    }
}
