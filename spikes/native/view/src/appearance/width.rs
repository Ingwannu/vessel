//! One numeric width bound/default owner. Following and legacy preference
//! intent belongs to prefs::ContentWidth, not a second nullable-width model.
use serde::{Deserialize, Serialize};

/// A concrete content maximum width, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct ContentPixels(u16);

impl ContentPixels {
    /// Smallest explicit content width.
    pub const MIN: u16 = 600;
    /// Largest explicit content width.
    pub const MAX: u16 = 4000;
    /// Current default, resolved afresh by a following preference.
    pub const DEFAULT: Self = Self(820);

    /// The validated width in pixels.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// An explicit width outside the source range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "content width must be {} to {} pixels",
    ContentPixels::MIN,
    ContentPixels::MAX
)]
pub struct WidthError;

impl TryFrom<u16> for ContentPixels {
    type Error = WidthError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(WidthError)
        }
    }
}
impl From<ContentPixels> for u16 {
    fn from(width: ContentPixels) -> Self {
        width.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn width_file_edge_keeps_the_same_source_bounds() {
        for rejected in [ContentPixels::MIN - 1, ContentPixels::MAX + 1] {
            assert!(serde_json::from_str::<ContentPixels>(&rejected.to_string()).is_err());
        }
        for valid in [
            ContentPixels::MIN,
            ContentPixels::DEFAULT.get(),
            ContentPixels::MAX,
        ] {
            let parsed: ContentPixels = serde_json::from_str(&valid.to_string()).unwrap();
            assert_eq!(parsed.get(), valid);
            assert_eq!(serde_json::to_string(&parsed).unwrap(), valid.to_string());
        }
    }
}
