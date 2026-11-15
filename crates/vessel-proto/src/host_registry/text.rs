//! Explicitly public display annotations are bounded before persistence and readable by local agents.
//! These fields must never be populated from private credential/configuration stores.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, str::FromStr};

/// Paseo's per-field host-label/metadata bounds, plus a directory admission floor.
pub const LABEL_CHARS: usize = 256;
/// Maximum UTF-16 code units in a metadata key (Paseo hosts.ts).
pub const METADATA_KEY_CHARS: usize = 256;
/// Maximum UTF-16 code units in a metadata value (Paseo hosts.ts).
pub const METADATA_VALUE_CHARS: usize = 4096;
/// Avoid an unbounded map even when every individual field is valid.
pub const METADATA_ENTRIES: usize = 256;
/// Aggregate annotation budget prevents a valid map from allocating megabytes per host.
pub const METADATA_BYTES: usize = 16 * 1024;

/// Invalid metadata never echoes its possibly sensitive contents.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidHostText {
    /// User text cannot include terminal control sequences.
    #[error("host text contains a control character")]
    Control,
    /// A bounded text field exceeded its declared admission limit.
    #[error("host text exceeds {maximum} characters")]
    TooLong {
        /// Field-specific limit, without its contents.
        maximum: usize,
    },
    /// A metadata map exceeded its count budget.
    #[error("host metadata exceeds {METADATA_ENTRIES} entries")]
    TooMany,
    /// A whole metadata map is bounded independently of each field.
    #[error("host metadata exceeds {METADATA_BYTES} bytes")]
    TooLarge,
}
macro_rules! text_type {
    ($name:ident, $limit:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);
        impl $name {
            /// Borrow validated text without copying registry metadata.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl FromStr for $name {
            type Err = InvalidHostText;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Self::try_from(text.to_owned())
            }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidHostText;
            fn try_from(text: String) -> Result<Self, Self::Error> {
                if text.chars().any(char::is_control) {
                    return Err(InvalidHostText::Control);
                }
                if text.encode_utf16().count() > $limit {
                    return Err(InvalidHostText::TooLong { maximum: $limit });
                }
                Ok(Self(text))
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
text_type!(
    HostLabel,
    LABEL_CHARS,
    "A display label never selects or authenticates a host."
);
text_type!(
    MetadataKey,
    METADATA_KEY_CHARS,
    "A bounded metadata key preserves a host source's annotations."
);
text_type!(
    MetadataValue,
    METADATA_VALUE_CHARS,
    "A bounded annotation carries text, never a credential or route."
);

/// A whole validated metadata map; updates cannot bypass the entry bound.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    try_from = "BTreeMap<MetadataKey, MetadataValue>",
    into = "BTreeMap<MetadataKey, MetadataValue>"
)]
pub struct HostMetadata(BTreeMap<MetadataKey, MetadataValue>);
impl TryFrom<BTreeMap<MetadataKey, MetadataValue>> for HostMetadata {
    type Error = InvalidHostText;
    fn try_from(value: BTreeMap<MetadataKey, MetadataValue>) -> Result<Self, Self::Error> {
        if value
            .iter()
            .try_fold(0_usize, |total, (key, value)| {
                total
                    .checked_add(key.as_str().len())?
                    .checked_add(value.as_str().len())
            })
            .is_none_or(|bytes| bytes > METADATA_BYTES)
        {
            return Err(InvalidHostText::TooLarge);
        }
        if value.len() > METADATA_ENTRIES {
            Err(InvalidHostText::TooMany)
        } else {
            Ok(Self(value))
        }
    }
}
impl From<HostMetadata> for BTreeMap<MetadataKey, MetadataValue> {
    fn from(value: HostMetadata) -> Self {
        value.0
    }
}
impl HostMetadata {
    /// Read annotations without permitting unchecked insertion.
    #[must_use]
    pub fn entries(&self) -> &BTreeMap<MetadataKey, MetadataValue> {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zod_utf16_bounds_and_aggregate_admission_are_preserved() {
        let emoji_count = LABEL_CHARS.checked_div(2).unwrap();
        assert!("😀".repeat(emoji_count).parse::<HostLabel>().is_ok());
        assert!(
            "😀"
                .repeat(emoji_count.checked_add(1).unwrap())
                .parse::<HostLabel>()
                .is_err()
        );
        assert!("line\ncontrol".parse::<HostLabel>().is_err());
        let entries = (0..METADATA_ENTRIES)
            .map(|i| {
                (
                    i.to_string().parse::<MetadataKey>().unwrap(),
                    "x".repeat(METADATA_VALUE_CHARS)
                        .parse::<MetadataValue>()
                        .unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            HostMetadata::try_from(entries).unwrap_err(),
            InvalidHostText::TooLarge
        );
    }
}
