//! Reading a JSON document this build only partly knows, and writing back
//! what it did not understand as it was.
//!
//! [Decision Log]
//! - Purpose: one preferences file serves every `vessel` a person runs,
//!   old and new. A newer page adds a setting, or a value an older bridge
//!   cannot read; the older bridge must neither fail the whole file nor
//!   drop what it did not understand when it writes its own change.
//! - Constraint: no `serde_json::Value` tree (AGENTS.md section 2). What
//!   is not understood is carried as [`RawJson`], opaque, and never looked
//!   into.
//! - Choice: two small types the documents are built from. A [`Field`] is
//!   one setting: absent, read, or kept raw because its value did not read
//!   (the page is given the default; a change replaces it). [`Entries`] is
//!   a map read entry by entry: an entry whose key or value does not read
//!   is kept raw under its key. A struct of `Field`s with a flattened
//!   `rest` map is a section that keeps its unknown keys.
//! - Alternatives: `deny_unknown_fields` and failing the file (a newer
//!   page's file would stop an older bridge); `#[serde(default)]` with
//!   unknown keys ignored (they would be dropped on the next write).
//! - Cost: a kept value is read twice (once as raw text, once as the
//!   typed value), and re-emitted compactly; the files are small (256 KiB
//!   and 1 MiB at most).
//! - Must stay true: a key is in a document's typed part or its kept part,
//!   never both, so a write emits each key once. Setting a typed value
//!   removes what was kept under its key.

use std::collections::BTreeMap;

use serde::de::{DeserializeOwned, Deserializer, IntoDeserializer as _};
use serde::ser::{SerializeMap as _, Serializer};
use serde::{Deserialize, Serialize};
use vessel_proto::RawJson;

/// One setting of a document: not there, read, or kept as it was because
/// it did not read. At most one of `value` and `kept` is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field<T> {
    value: Option<T>,
    kept: Option<RawJson>,
}

impl<T> Default for Field<T> {
    fn default() -> Self {
        Self {
            value: None,
            kept: None,
        }
    }
}

impl<T> Field<T> {
    /// A value read (or set).
    pub(crate) const fn new(value: T) -> Self {
        Self {
            value: Some(value),
            kept: None,
        }
    }

    /// The value read, if one was.
    pub(crate) const fn read(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Whether the document leaves it out (for `skip_serializing_if`).
    pub(crate) const fn is_absent(&self) -> bool {
        self.value.is_none() && self.kept.is_none()
    }

    /// Set it, replacing what was kept.
    pub(crate) fn set(&mut self, value: T) {
        *self = Self::new(value);
    }

    /// The value, replacing what was kept (or nothing) with `T`'s default
    /// first: what a change to a part of it edits.
    pub(crate) fn make_read(&mut self) -> &mut T
    where
        T: Default,
    {
        self.kept = None;
        self.value.get_or_insert_with(T::default)
    }
}

impl<T: Clone> Field<T> {
    /// The value read, else `default`.
    pub(crate) fn value_or(&self, default: T) -> T {
        self.value.clone().unwrap_or(default)
    }
}

impl<T: Blank> Field<T> {
    /// Whether writing it would say nothing: absent, or read and blank.
    pub(crate) fn is_blank(&self) -> bool {
        self.kept.is_none() && self.value.as_ref().is_none_or(Blank::is_blank)
    }
}

/// A section or a map that may hold nothing worth writing.
pub(crate) trait Blank {
    /// Whether it holds nothing.
    fn is_blank(&self) -> bool;
}

impl<T: Serialize> Serialize for Field<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match (&self.value, &self.kept) {
            (Some(value), _) => value.serialize(serializer),
            (None, Some(raw)) => raw.serialize(serializer),
            // Fields skip themselves when absent; one written anyway is
            // the setting's default, which is what `null` says.
            (None, None) => serializer.serialize_none(),
        }
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Field<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawJson::deserialize(deserializer)?;
        Ok(match serde_json::from_str(raw.as_text()) {
            Ok(value) => Self::new(value),
            Err(_unreadable) => Self {
                value: None,
                kept: Some(raw),
            },
        })
    }
}

/// A JSON object read entry by entry: entries whose key and value both
/// read, typed, and the others kept as they were.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entries<K, V> {
    read: BTreeMap<K, V>,
    kept: BTreeMap<String, RawJson>,
}

impl<K, V> Default for Entries<K, V> {
    fn default() -> Self {
        Self {
            read: BTreeMap::new(),
            kept: BTreeMap::new(),
        }
    }
}

impl<K, V> Blank for Entries<K, V> {
    fn is_blank(&self) -> bool {
        self.read.is_empty() && self.kept.is_empty()
    }
}

impl<K: Ord + DeserializeOwned, V> Entries<K, V> {
    /// The entries read.
    pub(crate) const fn read(&self) -> &BTreeMap<K, V> {
        &self.read
    }

    /// Every entry, read or kept.
    pub(crate) fn len(&self) -> usize {
        self.read.len().saturating_add(self.kept.len())
    }

    /// Set `key` to `value`, replacing whatever was kept under it.
    pub(crate) fn insert(&mut self, key: K, value: V) {
        self.forget_kept(&key);
        let _old = self.read.insert(key, value);
    }

    /// Remove `key`, read or kept.
    pub(crate) fn remove(&mut self, key: &K) {
        self.forget_kept(key);
        let _old = self.read.remove(key);
    }

    /// `key`'s value, a default one replacing whatever was kept under it
    /// when it was not read.
    pub(crate) fn make_read(&mut self, key: K) -> &mut V
    where
        V: Default,
    {
        if !self.read.contains_key(&key) {
            self.forget_kept(&key);
        }
        self.read.entry(key).or_default()
    }

    /// Remove `key` if `blank` says its value holds nothing.
    pub(crate) fn remove_if(&mut self, key: &K, blank: impl FnOnce(&V) -> bool) {
        if self.read.get(key).is_some_and(blank) {
            let _old = self.read.remove(key);
        }
    }

    /// Drop the kept entries under `key` (spelled the way it reads: a
    /// node id in capitals is the same node).
    fn forget_kept(&mut self, key: &K) {
        self.kept
            .retain(|text, _raw| parse_key::<K>(text).as_ref() != Some(key));
    }
}

/// `text` as a map key of type `K`, if it is one. Keys are read as text
/// (every key these files use is: action ids, node ids, agent kinds); a
/// key type read from a number would keep every entry raw.
fn parse_key<K: DeserializeOwned>(text: &str) -> Option<K> {
    let key: serde::de::value::StrDeserializer<'_, serde::de::value::Error> =
        text.into_deserializer();
    K::deserialize(key).ok()
}

impl<K: Serialize, V: Serialize> Serialize for Entries<K, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map =
            serializer.serialize_map(Some(self.read.len().saturating_add(self.kept.len())))?;
        for (key, value) in &self.read {
            map.serialize_entry(key, value)?;
        }
        for (key, raw) in &self.kept {
            map.serialize_entry(key, raw)?;
        }
        map.end()
    }
}

impl<'de, K: Ord + DeserializeOwned, V: DeserializeOwned> Deserialize<'de> for Entries<K, V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let all = BTreeMap::<String, RawJson>::deserialize(deserializer)?;
        let mut entries = Self::default();
        for (text, raw) in all {
            let value = serde_json::from_str::<V>(raw.as_text()).ok();
            match (parse_key::<K>(&text), value) {
                (Some(key), Some(value)) => {
                    let _twice = entries.read.insert(key, value);
                }
                (None | Some(_), None) | (None, Some(_)) => {
                    let _twice = entries.kept.insert(text, raw);
                }
            }
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::ActionId;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    struct Section {
        #[serde(skip_serializing_if = "Field::is_absent")]
        size: Field<u8>,
        #[serde(skip_serializing_if = "Field::is_blank")]
        keys: Field<Entries<ActionId, u8>>,
        #[serde(flatten)]
        rest: BTreeMap<String, RawJson>,
    }

    #[test]
    fn what_is_not_understood_is_written_back() {
        let text = r#"{"size":"huge","keys":{"one":1,"X":"ex","two":"2"},"later":{"a":[1,true]}}"#;
        let mut section: Section = serde_json::from_str(text).unwrap();
        assert!(section.size.read().is_none() && !section.size.is_absent());
        assert_eq!(section.size.value_or(7), 7);
        let keys = section.keys.read().unwrap();
        assert_eq!(keys.read().len(), 1);
        assert_eq!(keys.len(), 3);
        let written = serde_json::to_string(&section).unwrap();
        assert_eq!(
            written,
            r#"{"size":"huge","keys":{"one":1,"X":"ex","two":"2"},"later":{"a":[1,true]}}"#
        );
        // A change replaces what was kept under its key, and only that.
        section.size.set(3);
        section.keys.make_read().insert("two".parse().unwrap(), 2);
        let written = serde_json::to_string(&section).unwrap();
        assert_eq!(
            written,
            r#"{"size":3,"keys":{"one":1,"two":2,"X":"ex"},"later":{"a":[1,true]}}"#
        );
        // An absent section says nothing.
        assert_eq!(serde_json::to_string(&Section::default()).unwrap(), "{}");
    }
}
