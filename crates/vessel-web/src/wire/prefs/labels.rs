//! The sidebar's workspace label filter (the page's labels consumer of
//! ADR 0086 and ADR 0106): the label names the person picked.
//!
//! [Decision Log]
//! - Purpose: Paseo keeps the Labels filter of its sidebar view across
//!   launches, beside the grouping and the order. The page keeps it in the
//!   same preferences section.
//! - Choice: names as typed, the empty name meaning Unlabelled (Paseo's
//!   key for it), at most [`MAX_LABEL_PICKS`] of at most
//!   [`MAX_LABEL_PICK_BYTES`] each, so a page cannot grow the file without
//!   bound. A label is compared by its folded name on the page, so a name
//!   kept here survives a case-only rename on the machine.
//! - The filter belongs to the page, never to a node: it asks nothing of a
//!   machine.

use serde::{Deserialize, Deserializer, Serialize, de};

use super::Bounded;

/// The most label names the filter keeps.
pub const MAX_LABEL_PICKS: usize = 32;

/// The longest picked name, in bytes.
pub const MAX_LABEL_PICK_BYTES: usize = 256;

/// The picked names: at most [`MAX_LABEL_PICKS`]. In TypeScript an array
/// of strings (each field using it says so with `ts(as)`).
pub type LabelPicks = Bounded<LabelPick, MAX_LABEL_PICKS>;

/// One picked label name; empty is Unlabelled.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct LabelPick(String);

/// A picked name longer than [`MAX_LABEL_PICK_BYTES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a picked label name is longer than {MAX_LABEL_PICK_BYTES} bytes")]
pub struct LabelPickTooLong;

impl LabelPick {
    /// `name`, when it fits.
    ///
    /// # Errors
    /// Longer than [`MAX_LABEL_PICK_BYTES`].
    pub fn new(name: String) -> Result<Self, LabelPickTooLong> {
        if name.len() > MAX_LABEL_PICK_BYTES {
            Err(LabelPickTooLong)
        } else {
            Ok(Self(name))
        }
    }

    /// The name as picked.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LabelPick {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// No picks: every workspace is listed.
#[must_use]
pub(super) const fn no_picks() -> LabelPicks {
    Bounded::empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_are_bounded_in_count_and_length() {
        let long = "x".repeat(MAX_LABEL_PICK_BYTES + 1);
        assert!(serde_json::from_value::<LabelPick>(serde_json::json!(long)).is_err());
        let unlabelled: LabelPick = serde_json::from_value(serde_json::json!("")).unwrap();
        assert_eq!(unlabelled.as_str(), "");
        let many = vec![""; MAX_LABEL_PICKS + 1];
        assert!(serde_json::from_value::<LabelPicks>(serde_json::json!(many)).is_err());
        let picks: LabelPicks = serde_json::from_value(serde_json::json!(["Docs", ""])).unwrap();
        assert_eq!(picks.items().len(), 2);
    }
}
