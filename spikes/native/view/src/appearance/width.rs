//! Validated numeric content width. Default following is a choice, not a
//! serialized copy of today's default pixel count.
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

/// The nullable width preference's intent, used by reset and draft commits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Option<ContentPixels>", into = "Option<ContentPixels>")]
pub enum ContentMaximum {
    /// Keep following the application's current default.
    #[default]
    Default,
    /// A user's explicit validated width.
    Pixels(ContentPixels),
}
impl ContentMaximum {
    /// Resolve only when drawing; reset never stores today's default number.
    #[must_use]
    pub const fn resolved(self) -> ContentPixels {
        match self {
            Self::Default => ContentPixels::DEFAULT,
            Self::Pixels(value) => value,
        }
    }

    /// Committing the value already in effect preserves following semantics.
    #[must_use]
    pub fn commit(self, next: ContentPixels) -> Self {
        if self.resolved() == next {
            self
        } else {
            Self::Pixels(next)
        }
    }
}
impl From<Option<ContentPixels>> for ContentMaximum {
    fn from(value: Option<ContentPixels>) -> Self {
        match value {
            Some(value) => Self::Pixels(value),
            None => Self::Default,
        }
    }
}
impl From<ContentMaximum> for Option<ContentPixels> {
    fn from(value: ContentMaximum) -> Self {
        match value {
            ContentMaximum::Default => None,
            ContentMaximum::Pixels(value) => Some(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn width_is_nullable_bounded_and_same_value_does_not_pin_the_default() {
        let follow = ContentMaximum::default();
        assert_eq!(serde_json::to_string(&follow).unwrap(), "null");
        assert_eq!(follow.commit(follow.resolved()), follow);
        let wider = ContentPixels::try_from(ContentPixels::DEFAULT.get() + 1).unwrap();
        assert_eq!(follow.commit(wider), ContentMaximum::Pixels(wider));
        assert!(ContentPixels::try_from(ContentPixels::MIN - 1).is_err());
        assert!(ContentPixels::try_from(ContentPixels::MAX + 1).is_err());
        for width in [ContentPixels::MIN, ContentPixels::MAX] {
            let choice = ContentMaximum::Pixels(ContentPixels::try_from(width).unwrap());
            assert_eq!(
                serde_json::from_str::<ContentMaximum>(&serde_json::to_string(&choice).unwrap())
                    .unwrap(),
                choice
            );
        }
    }
}
