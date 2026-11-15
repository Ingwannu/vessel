//! Nonempty label changes at the decoding edge, without a second map validator.
//! Applying a change against current state remains SessionLabels' responsibility.

use serde::{Deserialize, Deserializer, Serialize, de};

use super::{InvalidLabel, SessionLabels};

/// A mutation must name a key even though an empty session state or filter is valid.
/// The private map cannot be emptied after its constructor has checked it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct LabelPatch(SessionLabels);

impl LabelPatch {
    /// Borrow validated changes for the existing atomic map operation and display,
    /// without giving a caller mutable access to the nonempty invariant.
    #[must_use]
    pub const fn labels(&self) -> &SessionLabels {
        &self.0
    }
}

impl TryFrom<SessionLabels> for LabelPatch {
    type Error = InvalidLabel;

    fn try_from(labels: SessionLabels) -> Result<Self, Self::Error> {
        if labels.is_empty() {
            return Err(InvalidLabel::Empty);
        }
        Ok(Self(labels))
    }
}

impl<'de> Deserialize<'de> for LabelPatch {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let labels = SessionLabels::deserialize(deserializer)?;
        Self::try_from(labels).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
