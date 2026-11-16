//! Intercept only the file's metadata value; leave ordinary fields on their tracked map.
//!
//! This is a decoding edge: foreign JSON keys are compared only to select the
//! one file-only value decoder. There is no second settings field table.

use std::fmt;

use serde::de::{
    DeserializeSeed, Error as _, MapAccess, Visitor,
    value::{MapAccessDeserializer, StringDeserializer},
};
use serde::{Deserialize, Deserializer};
use vessel_proto::settings::{MetadataSettings, SetNodeSettings};

/// Preserve the outer sealed decoder's path tracking without serde's flatten buffer.
pub(super) fn read<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<SetNodeSettings, D::Error> {
    deserializer.deserialize_map(SettingsVisitor)
}

struct SettingsVisitor;

impl<'de> Visitor<'de> for SettingsVisitor {
    type Value = SetNodeSettings;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("saved node settings")
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let mut file = FileMap {
            map,
            metadata: MetadataField::Missing,
        };
        let mut known = SetNodeSettings::deserialize(MapAccessDeserializer::new(&mut file))?;
        match file.metadata {
            MetadataField::Missing => {}
            MetadataField::Seen(value) => known.metadata = value,
        }
        Ok(known)
    }
}

enum MetadataField {
    Missing,
    Seen(Option<MetadataSettings>),
}

struct FileMap<A> {
    map: A,
    metadata: MetadataField,
}

impl<'de, A: MapAccess<'de>> MapAccess<'de> for FileMap<A> {
    type Error = A::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        while let Some(key) = self.map.next_key::<String>()? {
            if key == "metadata" {
                match self.metadata {
                    MetadataField::Missing => {}
                    MetadataField::Seen(_) => return Err(A::Error::duplicate_field("metadata")),
                }
                self.metadata = MetadataField::Seen(self.map.next_value_seed(MetadataSeed)?);
            } else {
                return seed.deserialize(StringDeserializer::new(key)).map(Some);
            }
        }
        Ok(None)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        self.map.next_value_seed(seed)
    }
}

struct MetadataSeed;

impl<'de> DeserializeSeed<'de> for MetadataSeed {
    type Value = Option<MetadataSettings>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        MetadataSettings::deserialize_optional_view(deserializer)
    }
}
