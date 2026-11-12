//! The bounded values of the preferences: sizes in their ranges, lists
//! and maps the page may not grow past their counts, and the remembered
//! choices of an agent. Each is refused (or, where the contract says so,
//! dropped) where the frame is decoded, so nothing past a bound reaches
//! the preferences task.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use vessel_proto::{ChoiceValue, HarnessKind, OptionId};

/// Declares a size in whole pixels, bounded and with a default.
macro_rules! pixels {
    ($(#[$meta:meta])* $name:ident, $min:literal ..= $max:literal, default $default:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(try_from = "u8", into = "u8")]
        pub struct $name(u8);

        impl $name {
            /// The smallest.
            pub const MIN: u8 = $min;
            /// The largest.
            pub const MAX: u8 = $max;

            /// The size in pixels.
            #[must_use]
            pub const fn get(self) -> u8 {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self($default)
            }
        }

        impl TryFrom<u8> for $name {
            type Error = OutOfRange;

            fn try_from(pixels: u8) -> Result<Self, OutOfRange> {
                if (Self::MIN..=Self::MAX).contains(&pixels) {
                    Ok(Self(pixels))
                } else {
                    Err(OutOfRange {
                        min: Self::MIN,
                        max: Self::MAX,
                    })
                }
            }
        }

        impl From<$name> for u8 {
            fn from(size: $name) -> Self {
                size.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

pixels! {
    /// The interface's text size: whole pixels, 12 to 20 (14 by default).
    InterfaceSize, 12..=20, default 14
}

pixels! {
    /// Code's text size: whole pixels, 10 to 18 (12 by default).
    CodeSize, 10..=18, default 12
}

/// A size outside its range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a size from {min} to {max} pixels")]
pub struct OutOfRange {
    min: u8,
    max: u8,
}

/// The longest option id or select value remembered for an agent, in
/// bytes.
pub const MAX_CHOICE_TEXT: usize = 64;

/// The most choices remembered for one agent on one machine.
pub const MAX_CHOICES: usize = 8;

/// An agent's option id remembered with its value: an [`OptionId`] of at
/// most [`MAX_CHOICE_TEXT`] bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "OptionId", into = "OptionId")]
pub struct ChoiceKey(OptionId);

/// An option id longer than [`MAX_CHOICE_TEXT`] bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("an option id of at most {MAX_CHOICE_TEXT} bytes")]
pub struct ChoiceKeyTooLong;

impl ChoiceKey {
    /// The option's id.
    #[must_use]
    pub const fn id(&self) -> &OptionId {
        &self.0
    }
}

impl TryFrom<OptionId> for ChoiceKey {
    type Error = ChoiceKeyTooLong;

    fn try_from(id: OptionId) -> Result<Self, ChoiceKeyTooLong> {
        if id.as_str().len() <= MAX_CHOICE_TEXT {
            Ok(Self(id))
        } else {
            Err(ChoiceKeyTooLong)
        }
    }
}

impl From<ChoiceKey> for OptionId {
    fn from(key: ChoiceKey) -> Self {
        key.0
    }
}

/// Whether `value` is short enough to remember.
fn choice_value_fits(value: &ChoiceValue) -> bool {
    match value {
        ChoiceValue::Select { value } => value.as_str().len() <= MAX_CHOICE_TEXT,
        ChoiceValue::Toggle { .. } => true,
    }
}

/// The choices remembered for one agent: at most [`MAX_CHOICES`], one
/// value per option. In TypeScript a map of [`ChoiceKey`] to
/// [`ChoiceValue`].
///
/// Decoding a change refuses more than [`MAX_CHOICES`] entries (the page
/// sent more than it may) and drops an entry whose id or select value is
/// longer than [`MAX_CHOICE_TEXT`] bytes: such a choice is not remembered,
/// which is what the contract asks, rather than the whole change failing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Choices(BTreeMap<ChoiceKey, ChoiceValue>);

impl Choices {
    /// No choices.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// The choices, by option.
    #[must_use]
    pub const fn get(&self) -> &BTreeMap<ChoiceKey, ChoiceValue> {
        &self.0
    }

    /// `choices` when they are within every bound, else why not. What the
    /// file holds is checked this way: a longer value a newer page kept is
    /// left as it was in the file rather than dropped.
    ///
    /// # Errors
    /// More than [`MAX_CHOICES`], or a value longer than
    /// [`MAX_CHOICE_TEXT`] bytes.
    pub fn strict(choices: BTreeMap<ChoiceKey, ChoiceValue>) -> Result<Self, ChoicesError> {
        if choices.len() > MAX_CHOICES {
            return Err(ChoicesError::TooMany);
        }
        if !choices.values().all(choice_value_fits) {
            return Err(ChoicesError::TooLong);
        }
        Ok(Self(choices))
    }
}

/// Why choices are refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ChoicesError {
    /// More than [`MAX_CHOICES`].
    #[error("at most {MAX_CHOICES} choices per agent")]
    TooMany,
    /// A select value longer than [`MAX_CHOICE_TEXT`] bytes.
    #[error("a choice's value is at most {MAX_CHOICE_TEXT} bytes")]
    TooLong,
}

impl<'de> Deserialize<'de> for Choices {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let sent = BTreeMap::<OptionId, ChoiceValue>::deserialize(deserializer)?;
        if sent.len() > MAX_CHOICES {
            return Err(de::Error::custom(ChoicesError::TooMany));
        }
        Ok(Self(
            sent.into_iter()
                .filter(|(_id, value)| choice_value_fits(value))
                .filter_map(|(id, value)| ChoiceKey::try_from(id).ok().map(|key| (key, value)))
                .collect(),
        ))
    }
}

/// At most `N` items: a list the page sends whose length is bounded,
/// refused where the frame is decoded when longer. In TypeScript an array
/// (each field using it says so with `ts(as)`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bounded<T, const N: usize>(Vec<T>);

impl<T, const N: usize> Bounded<T, N> {
    /// `items`, if there are at most `N`.
    ///
    /// # Errors
    /// More than `N`.
    pub fn new(items: Vec<T>) -> Result<Self, TooMany> {
        if items.len() > N {
            Err(TooMany { max: N })
        } else {
            Ok(Self(items))
        }
    }

    /// None.
    #[must_use]
    pub const fn empty() -> Self {
        Self(Vec::new())
    }

    /// The items.
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.0
    }
}

/// A list longer than its bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("at most {max} items")]
pub struct TooMany {
    max: usize,
}

impl<T: Serialize, const N: usize> Serialize for Bounded<T, N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>, const N: usize> Deserialize<'de> for Bounded<T, N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(Vec::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// An agent harness as a key of the remembered defaults: the protocol's
/// [`HarnessKind`], ordered by its name so the defaults can be a sorted
/// map (the protocol's type has no order of its own). In TypeScript it is
/// `HarnessKind` (each field using it says so with `ts(as)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentKind(pub HarnessKind);

impl Ord for AgentKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.name().cmp(other.0.name())
    }
}

impl PartialOrd for AgentKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::OptionValueId;

    use super::*;

    #[test]
    fn sizes_are_whole_pixels_in_range() {
        assert_eq!(InterfaceSize::default().get(), 14);
        assert_eq!(CodeSize::default().get(), 12);
        assert_eq!(
            serde_json::from_str::<InterfaceSize>("20").unwrap().get(),
            20
        );
        for bad in ["11", "21", "14.5", "-1", "300", "\"14\""] {
            assert!(serde_json::from_str::<InterfaceSize>(bad).is_err(), "{bad}");
        }
        assert_eq!(serde_json::from_str::<CodeSize>("10").unwrap().get(), 10);
        assert!(serde_json::from_str::<CodeSize>("19").is_err());
    }

    #[test]
    fn choices_refuse_too_many_and_drop_what_is_too_long() {
        let long = "x".repeat(MAX_CHOICE_TEXT.saturating_add(1));
        let text = format!(
            r#"{{"model":{{"kind":"select","value":"opus"}},"fast":{{"kind":"toggle","on":true}},
                "{long}":{{"kind":"toggle","on":false}},"effort":{{"kind":"select","value":"{long}"}}}}"#
        );
        let choices: Choices = serde_json::from_str(&text).unwrap();
        let ids: Vec<&str> = choices.get().keys().map(|key| key.id().as_str()).collect();
        assert_eq!(ids, ["fast", "model"]);
        let nine: BTreeMap<String, ChoiceValue> = (0..9)
            .map(|n| (format!("o{n}"), ChoiceValue::Toggle { on: true }))
            .collect();
        let nine = serde_json::to_string(&nine).unwrap();
        assert!(serde_json::from_str::<Choices>(&nine).is_err());

        let key = ChoiceKey::try_from(OptionId::new("effort")).unwrap();
        let too_long = ChoiceValue::Select {
            value: OptionValueId::new(long.as_str()),
        };
        assert_eq!(
            Choices::strict(BTreeMap::from([(key, too_long)])),
            Err(ChoicesError::TooLong)
        );
    }

    #[test]
    fn bounded_lists_refuse_past_their_bound() {
        assert!(serde_json::from_str::<Bounded<u8, 2>>("[1,2]").is_ok());
        assert!(serde_json::from_str::<Bounded<u8, 2>>("[1,2,3]").is_err());
        let agents = [
            AgentKind(HarnessKind::OpenCode),
            AgentKind(HarnessKind::ClaudeCode),
        ];
        assert!(agents.iter().max() == Some(&AgentKind(HarnessKind::OpenCode)));
    }
}
