//! Validated session labels and bounded map operations (ADR 0063).
//! Wire versioning, discovery requests and session lifecycle belong elsewhere.
//!
//! Keys are trimmed and use [`LabelKey::MAX_CHARS`]; values retain their
//! whitespace and use [`LabelValue::MAX_CHARS`]. Both reject control
//! characters, including C1. The map cap is [`SessionLabels::MAX`]. Keys
//! cannot contain `=` and the `vessel.` prefix is reserved.
//!
//! [`SessionTitle`]: crate::title::SessionTitle

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};

use crate::key::serde_via_text;

mod patch;
pub use patch::LabelPatch;

/// A label's key. The constructor is the only way to build one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct LabelKey(Arc<str>);

/// A label's value. Empty is allowed; it is not trimmed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct LabelValue(Arc<str>);

/// Why a label was refused. The text names the rule, and a key only when
/// that key is safe to print (it passed the control and length checks).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidLabel {
    /// Nothing but white space.
    #[error("a label key is empty")]
    EmptyKey,
    /// A control character, including C1. The text is not echoed.
    #[error("a label cannot contain a control character")]
    Control,
    /// More than [`LabelKey::MAX_CHARS`] scalars, after trimming.
    #[error("a label key exceeds {} Unicode scalars", LabelKey::MAX_CHARS)]
    KeyTooLong,
    /// More than [`LabelValue::MAX_CHARS`] scalars.
    #[error("a label value exceeds {} Unicode scalars", LabelValue::MAX_CHARS)]
    ValueTooLong,
    /// The key contains `=`.
    #[error("label key '{key}' cannot contain '='")]
    Equals {
        /// The key, which is safe to print.
        key: String,
    },
    /// The key starts with `vessel.`.
    #[error("label key '{key}' is reserved")]
    Reserved {
        /// The key, which is safe to print.
        key: String,
    },
    /// The session would have more than [`SessionLabels::MAX`] labels.
    #[error("a session cannot hold more than {} labels", SessionLabels::MAX)]
    TooMany,
    /// A patch names no key.
    #[error("nothing to update")]
    Empty,
}

impl LabelKey {
    /// The longest key, in Unicode scalars.
    pub const MAX_CHARS: usize = 64;

    /// The key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(raw: &str) -> Result<Self, InvalidLabel> {
        if raw.chars().any(char::is_control) {
            return Err(InvalidLabel::Control);
        }
        let text = raw.trim();
        if text.is_empty() {
            return Err(InvalidLabel::EmptyKey);
        }
        let chars = text.chars().count();
        if chars > Self::MAX_CHARS {
            return Err(InvalidLabel::KeyTooLong);
        }
        if text.contains('=') {
            return Err(InvalidLabel::Equals {
                key: text.to_owned(),
            });
        }
        if text.starts_with("vessel.") {
            return Err(InvalidLabel::Reserved {
                key: text.to_owned(),
            });
        }
        Ok(Self(Arc::from(text)))
    }
}

impl LabelValue {
    /// The longest value, in Unicode scalars.
    pub const MAX_CHARS: usize = 256;

    /// The value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(raw: &str) -> Result<Self, InvalidLabel> {
        if raw.chars().any(char::is_control) {
            return Err(InvalidLabel::Control);
        }
        let chars = raw.chars().count();
        if chars > Self::MAX_CHARS {
            return Err(InvalidLabel::ValueTooLong);
        }
        Ok(Self(Arc::from(raw)))
    }
}

impl FromStr for LabelKey {
    type Err = InvalidLabel;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl FromStr for LabelValue {
    type Err = InvalidLabel;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl fmt::Display for LabelKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for LabelValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

serde_via_text!(LabelKey, LabelValue);

/// The labels a session carries now, or the ones a creation asked for.
///
/// Keys are ordered, so a summary and a stored row do not depend on who
/// sent the map. An empty map is left off the wire.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "Record<string, string>"))]
pub struct SessionLabels(BTreeMap<LabelKey, LabelValue>);

impl SessionLabels {
    /// How many labels one session holds.
    pub const MAX: usize = 32;

    /// No labels.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Whether it names nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Drop every label. An older peer is shown a session with none.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Set `key` to `value`. The same key again keeps the later value.
    /// A new key past [`MAX`](Self::MAX) is refused and this map stays.
    ///
    /// # Errors
    /// [`InvalidLabel::TooMany`] when the map is already full.
    pub fn insert(&mut self, key: LabelKey, value: LabelValue) -> Result<(), InvalidLabel> {
        if !self.0.contains_key(&key) && self.0.len() == Self::MAX {
            return Err(InvalidLabel::TooMany);
        }
        let _replaced = self.0.insert(key, value);
        Ok(())
    }

    /// The map after `patch`. Omitted keys stay. An empty patch is refused
    /// and this map is not copied into the result. A patch that would pass
    /// [`MAX`](Self::MAX) is refused and this map stays as it is.
    ///
    /// # Errors
    /// [`InvalidLabel::Empty`] when `patch` names nothing, or
    /// [`InvalidLabel::TooMany`] when the result would not fit.
    pub fn apply(&self, patch: &Self) -> Result<Self, InvalidLabel> {
        if patch.is_empty() {
            return Err(InvalidLabel::Empty);
        }
        let mut next = self.clone();
        for (key, value) in &patch.0 {
            next.insert(key.clone(), value.clone())?;
        }
        Ok(next)
    }

    /// Whether this map holds every pair in `required`. Empty `required`
    /// matches every session: a list with no filter is the whole list.
    #[must_use]
    pub fn covers(&self, required: &Self) -> bool {
        required
            .0
            .iter()
            .all(|(key, value)| self.0.get(key) == Some(value))
    }

    /// `key=value` pairs in key order, for a log line and a list. Values
    /// may contain commas; a control character never reaches here.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut line = String::new();
        for (key, value) in &self.0 {
            if !line.is_empty() {
                line.push_str(", ");
            }
            line.push_str(key.as_str());
            line.push('=');
            line.push_str(value.as_str());
        }
        line
    }
}

impl Serialize for SessionLabels {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SessionLabels {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct LabelsVisitor;
        impl<'de> Visitor<'de> for LabelsVisitor {
            type Value = SessionLabels;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "at most {} validated string labels", SessionLabels::MAX)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut input: A) -> Result<Self::Value, A::Error> {
                let mut labels = SessionLabels::new();
                // Enforce the bound while decoding, before an unbounded map exists.
                while let Some((key, value)) = input.next_entry()? {
                    labels.insert(key, value).map_err(de::Error::custom)?;
                }
                Ok(labels)
            }
        }
        deserializer.deserialize_map(LabelsVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(pairs: &[(&str, &str)]) -> SessionLabels {
        let mut map = SessionLabels::new();
        for (key, value) in pairs {
            map.insert(key.parse().unwrap(), value.parse().unwrap())
                .unwrap();
        }
        map
    }

    #[test]
    fn a_label_refuses_every_control_character_including_u_0085() {
        assert_eq!("\u{0085}".parse::<LabelKey>(), Err(InvalidLabel::Control));
        assert_eq!("\u{0085}".parse::<LabelValue>(), Err(InvalidLabel::Control));
        assert_eq!("a\u{0000}b".parse::<LabelKey>(), Err(InvalidLabel::Control));
        assert_eq!("a\nb".parse::<LabelValue>(), Err(InvalidLabel::Control));
        assert!("a\u{00A0}b".parse::<LabelKey>().is_ok());
    }

    #[test]
    fn a_key_is_trimmed_and_a_value_is_not() {
        assert_eq!("  k  ".parse::<LabelKey>().unwrap().as_str(), "k");
        assert_eq!("  x  ".parse::<LabelValue>().unwrap().as_str(), "  x  ");
        assert_eq!("   ".parse::<LabelKey>(), Err(InvalidLabel::EmptyKey));
        assert!("".parse::<LabelValue>().is_ok());
    }

    #[test]
    fn bounds_equals_and_the_reserved_prefix_are_refused() {
        assert!("한".repeat(LabelKey::MAX_CHARS).parse::<LabelKey>().is_ok());
        assert!(matches!(
            "a".repeat(LabelKey::MAX_CHARS.saturating_add(1))
                .parse::<LabelKey>(),
            Err(InvalidLabel::KeyTooLong)
        ));
        assert!(
            "값"
                .repeat(LabelValue::MAX_CHARS)
                .parse::<LabelValue>()
                .is_ok()
        );
        assert!(matches!(
            "v".repeat(LabelValue::MAX_CHARS.saturating_add(1))
                .parse::<LabelValue>(),
            Err(InvalidLabel::ValueTooLong)
        ));
        assert!(matches!(
            "a=b".parse::<LabelKey>(),
            Err(InvalidLabel::Equals { .. })
        ));
        assert!(matches!(
            "vessel.env".parse::<LabelKey>(),
            Err(InvalidLabel::Reserved { .. })
        ));
        assert!("vessel".parse::<LabelKey>().is_ok());
    }

    #[test]
    fn a_patch_replaces_named_keys_and_refuses_empty_or_overflow() {
        let current = labels(&[("a", "1"), ("b", "2")]);
        let patched = current.apply(&labels(&[("b", "3"), ("c", "4")])).unwrap();
        assert_eq!(patched, labels(&[("a", "1"), ("b", "3"), ("c", "4")]));
        assert_eq!(
            current.apply(&SessionLabels::new()),
            Err(InvalidLabel::Empty)
        );
        assert_eq!(InvalidLabel::Empty.to_string(), "nothing to update");
        let mut full = SessionLabels::new();
        for index in 0..SessionLabels::MAX {
            full.insert(
                format!("k{index:02}").parse().unwrap(),
                "v".parse().unwrap(),
            )
            .unwrap();
        }
        assert_eq!(full.0.len(), SessionLabels::MAX);
        let before = full.clone();
        let err = full.apply(&labels(&[("new", "x")])).unwrap_err();
        assert!(matches!(err, InvalidLabel::TooMany));
        assert_eq!(full, before);
        full.insert("k00".parse().unwrap(), "z".parse().unwrap())
            .unwrap();
        assert_eq!(full.0.len(), SessionLabels::MAX);
    }

    #[test]
    fn overflow_does_not_decode_and_a_filter_is_a_logical_and() {
        let mut object = serde_json::Map::new();
        for index in 0..SessionLabels::MAX.saturating_add(1) {
            let _replaced = object.insert(
                format!("k{index:02}"),
                serde_json::Value::String("v".to_owned()),
            );
        }
        assert!(
            serde_json::from_value::<SessionLabels>(serde_json::Value::Object(object)).is_err()
        );
        let map = labels(&[("env", "prod"), ("tier", "a")]);
        assert!(map.covers(&labels(&[("env", "prod"), ("tier", "a")])));
        assert!(!map.covers(&labels(&[("env", "prod"), ("tier", "b")])));
        assert!(map.covers(&labels(&[("env", "prod")])));
        assert!(!map.covers(&labels(&[("env", "dev")])));
        assert!(map.covers(&SessionLabels::new()));
        let text = serde_json::to_string(&map).unwrap();
        assert_eq!(serde_json::from_str::<SessionLabels>(&text).unwrap(), map);
        assert_eq!(map.summary(), "env=prod, tier=a");
    }
}
