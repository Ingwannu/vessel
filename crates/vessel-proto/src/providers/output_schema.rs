//! Bounded opaque JSON Schema content for an initial prompt. Validation and
//! retries belong to the caller/vendor; provider policy only admits a real
//! native transport. The wire carries JSON text, keeping opaque content distinct
//! from the protocol's own typed fields and avoiding a second schema AST.

use crate::RawJson;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{fmt, str::FromStr};

/// Bound schema content before parsing or retaining a native JSON buffer.
pub const MAX_OUTPUT_SCHEMA_BYTES: usize = 64 * 1024;

/// Valid JSON object content, with no field inspection or I/O authority.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(type = "string"))]
pub struct OutputSchema(RawJson);

impl OutputSchema {
    /// The text protocol representation, for an editor or CLI validator.
    #[must_use]
    pub fn as_text(&self) -> &str {
        self.0.as_text()
    }

    /// Use only at a vendor encoding boundary which supports this capability.
    #[must_use]
    pub const fn native_json(&self) -> &RawJson {
        &self.0
    }
}

/// Safe summary; schema defaults/examples may contain private user content.
#[derive(thiserror::Error)]
pub enum OutputSchemaError {
    /// A fixed resource limit, independent from transport frame bounds.
    #[error("output schema exceeds {MAX_OUTPUT_SCHEMA_BYTES} bytes")]
    TooLarge,
    /// Native providers take schema objects, not a scalar or boolean schema.
    #[error("output schema must be a JSON object")]
    ObjectRequired,
    /// Retain the decoder cause without including content in Display/Debug.
    #[error("output schema is not valid JSON")]
    Decode(#[source] serde_json::Error),
}

impl fmt::Debug for OutputSchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Debug for OutputSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputSchema")
            .field("bytes", &self.as_text().len())
            .finish_non_exhaustive()
    }
}

impl FromStr for OutputSchema {
    type Err = OutputSchemaError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > MAX_OUTPUT_SCHEMA_BYTES {
            return Err(OutputSchemaError::TooLarge);
        }
        let json = RawJson::from_text(text).map_err(OutputSchemaError::Decode)?;
        // The parser has proved a single valid JSON value. Its first token
        // identifies the root kind without inspecting any schema property.
        if !json.as_text().trim_start().starts_with('{') {
            return Err(OutputSchemaError::ObjectRequired);
        }
        Ok(Self(json))
    }
}

impl Serialize for OutputSchema {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_text())
    }
}

impl<'de> Deserialize<'de> for OutputSchema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Text;
        impl de::Visitor<'_> for Text {
            type Value = OutputSchema;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded JSON object schema text")
            }
            fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
                text.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_str(Text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_content_and_native_object_remain_distinct_and_redacted() {
        let text = r#"{"type":"object","properties":{"x":{"const":"synthetic-private"}}}"#;
        let schema: OutputSchema = text.parse().unwrap();
        let wire = serde_json::to_string(&schema).unwrap();
        assert_eq!(serde_json::from_str::<OutputSchema>(&wire).unwrap(), schema);
        assert_eq!(serde_json::from_str::<String>(&wire).unwrap(), text);
        assert_eq!(serde_json::to_string(schema.native_json()).unwrap(), text);
        assert!(!format!("{schema:?}").contains("synthetic-private"));
    }
    #[test]
    fn schema_bounds_and_root_kind_are_checked_before_native_delivery() {
        for text in ["null", "true", "[]", "42", "\"scalar\""] {
            assert!(matches!(
                text.parse::<OutputSchema>(),
                Err(OutputSchemaError::ObjectRequired)
            ));
        }
        assert!("{bad".parse::<OutputSchema>().is_err());
        let oversized = format!(
            "{{\"default\":\"{}\"}}",
            "x".repeat(MAX_OUTPUT_SCHEMA_BYTES)
        );
        assert!(matches!(
            oversized.parse::<OutputSchema>(),
            Err(OutputSchemaError::TooLarge)
        ));
        assert!(" {} ".parse::<OutputSchema>().is_ok());
        assert!(serde_json::from_str::<OutputSchema>(r#"{"type":"object"}"#).is_err());
    }
}
