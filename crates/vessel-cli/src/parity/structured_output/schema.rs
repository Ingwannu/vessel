//! The single approved JSON Schema edge: jsonschema owns interpretation.
//! Value is confined here because its compiler requires that representation.
//! Only bounded `RawJson` crosses this module; no field traversal or payload diagnostics.
#![allow(
    clippy::disallowed_types,
    reason = "coordinator-approved CLI JSON Schema compiler edge; no Value escapes"
)]

use serde::Deserialize;
use vessel_proto::RawJson;

pub(super) const MAX_SCHEMA_BYTES: usize = 65_536;
pub(super) const MAX_RESPONSE_BYTES: usize = 1_048_576;

#[derive(Default, Deserialize)]
struct DialectHeader {
    #[serde(default, rename = "$schema")]
    dialect: SupportedDialect,
}
#[derive(Default, Deserialize)]
enum SupportedDialect {
    #[default]
    #[serde(
        rename = "http://json-schema.org/draft-07/schema#",
        alias = "http://json-schema.org/draft-07/schema"
    )]
    Draft7,
}
impl SupportedDialect {
    fn version(self) -> jsonschema::Draft {
        match self {
            Self::Draft7 => jsonschema::Draft::Draft7,
        }
    }
}

pub(super) struct Schema {
    document: RawJson,
    validator: jsonschema::Validator,
}
#[derive(Debug, thiserror::Error)]
pub(super) enum SchemaError {
    #[error("output schema exceeds the CLI byte budget")]
    TooLarge,
    #[error("output schema must be a JSON object")]
    NotObject,
    #[error("output schema declares an unsupported dialect; use draft-07 or omit $schema")]
    Dialect,
    #[error("output schema is invalid or requires unavailable external references")]
    Invalid,
}
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum ResponseError {
    #[error("structured response exceeds the CLI byte budget")]
    TooLarge,
    #[error("structured response does not match the required schema")]
    Mismatch,
}
impl Schema {
    pub(super) fn compile(document: RawJson) -> Result<Self, SchemaError> {
        if document.as_text().len() > MAX_SCHEMA_BYTES {
            return Err(SchemaError::TooLarge);
        }
        let parsed: serde_json::Value =
            serde_json::from_str(document.as_text()).map_err(|_invalid| SchemaError::Invalid)?;
        // This root-kind gate mirrors Paseo loadOutputSchema, not schema traversal.
        if !parsed.is_object() {
            return Err(SchemaError::NotObject);
        }
        // Paseo imports Ajv's draft-07 class without ajv-formats. Parse the
        // declared dialect with a closed type, never by traversing Value.
        let header: DialectHeader =
            serde_json::from_str(document.as_text()).map_err(|_dialect| SchemaError::Dialect)?;
        let validator = jsonschema::options()
            .with_draft(header.dialect.version())
            .should_validate_formats(false)
            .offline()
            .build(&parsed)
            .map_err(|_invalid| SchemaError::Invalid)?;
        Ok(Self {
            document,
            validator,
        })
    }
    pub(super) fn document(&self) -> &RawJson {
        &self.document
    }
    pub(super) fn validate(&self, response: RawJson) -> Result<RawJson, ResponseError> {
        if response.as_text().len() > MAX_RESPONSE_BYTES {
            return Err(ResponseError::TooLarge);
        }
        let parsed: serde_json::Value =
            serde_json::from_str(response.as_text()).map_err(|_invalid| ResponseError::Mismatch)?;
        if !self.validator.is_valid(&parsed) {
            return Err(ResponseError::Mismatch);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn raw(text: &str) -> RawJson {
        RawJson::from_text(text).unwrap()
    }
    #[test]
    fn schema_validates_semantics_without_losing_typed_json_output() {
        let schema = Schema::compile(raw(r#"{"type":"object","properties":{"count":{"type":"integer","minimum":2}},"required":["count"],"additionalProperties":false}"#)).unwrap();
        let input = raw(r#"{"count":3}"#);
        assert_eq!(schema.validate(input.clone()).unwrap(), input);
        for invalid in [
            r#"{"count":1}"#,
            r#"{"count":"3"}"#,
            r#"{"count":3,"extra":true}"#,
            "{}",
        ] {
            assert_eq!(
                schema.validate(raw(invalid)).unwrap_err(),
                ResponseError::Mismatch
            );
        }
    }
    #[test]
    fn schema_refuses_all_external_retrieval_but_keeps_internal_refs() {
        for external in [
            r#"{"$ref":"https://example.invalid/schema.json"}"#,
            r#"{"$ref":"file:///must-not-read.json"}"#,
        ] {
            assert!(matches!(
                Schema::compile(raw(external)),
                Err(SchemaError::Invalid)
            ));
        }
        let internal = Schema::compile(raw(
            r##"{"definitions":{"word":{"type":"string"}},"$ref":"#/definitions/word"}"##,
        ))
        .unwrap();
        assert!(internal.validate(raw(r#""ok""#)).is_ok());
        assert_eq!(
            internal.validate(raw("false")).unwrap_err(),
            ResponseError::Mismatch
        );
        assert!(matches!(
            Schema::compile(raw("true")),
            Err(SchemaError::NotObject)
        ));
        assert!(matches!(
            Schema::compile(raw(r#"{"type":42}"#)),
            Err(SchemaError::Invalid)
        ));
    }
    #[test]
    fn schema_uses_paseo_draft7_and_refuses_other_declared_dialects() {
        // Tuple items are draft-07; the library's default 2020-12 rejects them.
        let tuple = Schema::compile(raw(
            r#"{"type":"array","items":[{"type":"integer"}],"additionalItems":false}"#,
        ))
        .unwrap();
        assert!(tuple.validate(raw("[1]")).is_ok());
        assert_eq!(
            tuple.validate(raw("[1,2]")).unwrap_err(),
            ResponseError::Mismatch
        );
        let format = Schema::compile(raw(r#"{"type":"string","format":"email"}"#)).unwrap();
        assert!(format.validate(raw(r#""not-an-email""#)).is_ok());
        for unsupported in [
            r#"{"$schema":"https://json-schema.org/draft/2020-12/schema"}"#,
            r#"{"$schema":null}"#,
        ] {
            assert!(matches!(
                Schema::compile(raw(unsupported)),
                Err(SchemaError::Dialect)
            ));
        }
    }
    #[test]
    fn schema_and_response_budgets_fail_before_interpretation_and_errors_are_redacted() {
        let large = format!("{{\"description\":\"{}\"}}", "s".repeat(MAX_SCHEMA_BYTES));
        assert!(matches!(
            Schema::compile(raw(&large)),
            Err(SchemaError::TooLarge)
        ));
        let schema = Schema::compile(raw("{}")).unwrap();
        let large = format!("\"{}\"", "r".repeat(MAX_RESPONSE_BYTES));
        assert_eq!(
            schema.validate(raw(&large)).unwrap_err(),
            ResponseError::TooLarge
        );
        let secret = r#"{"$ref":"file:///private-content-must-not-appear"}"#;
        let Err(error) = Schema::compile(raw(secret)) else {
            panic!("external reference admitted")
        };
        assert!(!error.to_string().contains("private-content"));
    }
}
