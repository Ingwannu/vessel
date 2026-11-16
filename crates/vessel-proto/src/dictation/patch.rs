//! Presence belongs to the field; null resets it and a value selects it.
//!
//! Deserialize through an option visitor instead of an untagged trial decoder:
//! a failed value keeps its validator's diagnostic rather than losing it while
//! trying another variant. The outer field's default distinguishes omission.

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, Visitor},
};
use std::{fmt, marker::PhantomData};

/// A present dictation patch field: a selected value or explicit reset to default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(untagged)]
pub enum DictationChange<T> {
    /// Keep a validated selection; it need not be encoded or parsed again.
    Set(T),
    /// Null removes the saved override, following Paseo's field reset.
    Reset,
}

impl<T> DictationChange<T> {
    /// A reset resolves through the default instead of becoming a saved value.
    pub(crate) fn value(&self) -> Option<&T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Reset => None,
        }
    }
    /// Stored settings keep selections, with reset operations normalized away.
    pub(crate) fn overlay(change: Option<Self>, saved: Option<Self>) -> Option<Self> {
        match change.or(saved) {
            Some(value @ Self::Set(_)) => Some(value),
            Some(Self::Reset) | None => None,
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for DictationChange<T> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        decoder.deserialize_option(Change(PhantomData))
    }
}

struct Change<T>(PhantomData<T>);
impl<'de, T: Deserialize<'de>> Visitor<'de> for Change<T> {
    type Value = DictationChange<T>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a selected value or null to reset it")
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(DictationChange::Reset)
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(DictationChange::Reset)
    }
    fn visit_some<D: Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        T::deserialize(decoder).map(DictationChange::Set)
    }
}

/// A present null must be decoded as Reset, never swallowed by outer Option.
pub(crate) fn present<'de, D, T>(decoder: D) -> Result<Option<DictationChange<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    DictationChange::deserialize(decoder).map(Some)
}
