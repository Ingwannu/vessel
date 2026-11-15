//! YAML is decoded once into a document-format AST, not opaque JSON or
//! executable plugin data. Ordered fields preserve the author's table order.

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Metadata {
    Null,
    Boolean(bool),
    Signed(i64),
    Unsigned(u64),
    Float(f64),
    Text(String),
    Sequence(Vec<Self>),
    Mapping(Fields),
}

#[derive(Debug)]
struct Fields(Vec<(String, Metadata)>);

impl Serialize for Fields {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl Metadata {
    fn scalar(&self) -> Option<String> {
        Some(match self {
            Self::Null => "null".to_owned(),
            Self::Boolean(value) => value.to_string(),
            Self::Signed(value) => value.to_string(),
            Self::Unsigned(value) => value.to_string(),
            Self::Float(value) if value.is_nan() => "NaN".to_owned(),
            Self::Float(value) if value.is_infinite() => if value.is_sign_negative() {
                "-Infinity"
            } else {
                "Infinity"
            }
            .to_owned(),
            Self::Float(value) => value.to_string(),
            Self::Text(value) => value.clone(),
            Self::Sequence(_) | Self::Mapping(_) => return None,
        })
    }

    fn display(&self) -> Option<String> {
        if let Some(text) = self.scalar() {
            return Some(text);
        }
        if let Self::Sequence(values) = self
            && let Some(values) = values.iter().map(Self::scalar).collect::<Option<Vec<_>>>()
        {
            return Some(values.join(", "));
        }
        serde_saphyr::to_string(self)
            .ok()
            .map(|text| text.trim().to_owned())
    }
}

impl<'de> Deserialize<'de> for Metadata {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(MetadataVisitor)
    }
}

struct MetadataVisitor;
macro_rules! scalar {
    ($method:ident, $kind:ty, $variant:ident) => {
        fn $method<E: de::Error>(self, value: $kind) -> Result<Self::Value, E> {
            Ok(Metadata::$variant(value))
        }
    };
}
impl<'de> Visitor<'de> for MetadataVisitor {
    type Value = Metadata;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded YAML front matter")
    }
    scalar!(visit_bool, bool, Boolean);
    scalar!(visit_i64, i64, Signed);
    scalar!(visit_u64, u64, Unsigned);
    scalar!(visit_f64, f64, Float);
    scalar!(visit_string, String, Text);
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Metadata::Null)
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Metadata::Null)
    }
    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        Ok(Metadata::Text(text.to_owned()))
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut sequence: S) -> Result<Self::Value, S::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(Metadata::Sequence(values))
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let mut fields = Vec::new();
        while let Some(key) = map.next_key::<Metadata>()? {
            let key = key
                .scalar()
                .ok_or_else(|| de::Error::custom("front matter keys must be scalar"))?;
            fields.push((key, map.next_value()?));
        }
        Ok(Metadata::Mapping(Fields(fields)))
    }
}

/// Fail closed to the original Markdown source on invalid/non-map metadata.
pub(super) fn rows(source: &str) -> Option<Vec<super::FrontMatterRow>> {
    let options = serde_saphyr::options! {
        merge_keys: serde_saphyr::MergeKeyPolicy::AsOrdinary,
        budget: serde_saphyr::budget! {
            max_reader_input_bytes: Some(super::MAX_FRONT_MATTER_BYTES),
            max_documents: 1,
            max_aliases: super::MAX_FRONT_MATTER_ALIASES,
            max_anchors: super::MAX_FRONT_MATTER_ALIASES,
            max_events: super::MAX_FRONT_MATTER_EVENTS,
            max_nodes: super::MAX_FRONT_MATTER_EVENTS,
            max_recorded_anchor_events: super::MAX_FRONT_MATTER_EVENTS,
            max_recorded_anchor_bytes: super::MAX_FRONT_MATTER_BYTES,
            max_total_scalar_bytes: super::MAX_FRONT_MATTER_BYTES,
            max_depth: super::MAX_FRONT_MATTER_DEPTH,
            flow_nesting_limit: super::MAX_FRONT_MATTER_DEPTH,
            max_merge_keys: 0,
        },
        alias_limits: serde_saphyr::alias_limits! {
            max_total_replayed_events: super::MAX_FRONT_MATTER_EVENTS,
            max_alias_expansions_per_anchor: super::MAX_FRONT_MATTER_ALIASES,
            max_replay_stack_depth: super::MAX_FRONT_MATTER_DEPTH,
        },
    };
    let Metadata::Mapping(fields) =
        serde_saphyr::from_str_with_options::<Metadata>(source, options).ok()?
    else {
        return None;
    };
    fields
        .0
        .into_iter()
        .map(|(key, value)| {
            Some(super::FrontMatterRow {
                key,
                value: value.display()?,
            })
        })
        .collect()
}
