//! Decode secret names without echoing an accidentally pasted credential.
//! Owns only the field boundary; it never reads or resolves a secret value.

use super::DictationDecodeError;
use crate::name::{InvalidName, NameKind, SecretName};
use serde::{
    Deserializer,
    de::{self, Visitor},
};
use std::{error::Error, fmt};

/// A selected node secret's name; its owner checks existence before saving it.
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(transparent)]
pub struct SecretReference(#[serde(deserialize_with = "deserialize")] SecretName);

impl From<SecretName> for SecretReference {
    fn from(name: SecretName) -> Self {
        Self(name)
    }
}
impl SecretReference {
    /// Keep actual lookup in the existing secret owner.
    pub(crate) fn selected_name(&self) -> &SecretName {
        &self.0
    }
}
impl fmt::Debug for SecretReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretReference([redacted])")
    }
}

/// Retain the validator's typed cause after removing its submitted text.
#[derive(Debug)]
pub struct RedactedNameError(InvalidName);

impl From<InvalidName> for RedactedNameError {
    fn from(mut source: InvalidName) -> Self {
        source.value = String::from("[redacted]");
        Self(source)
    }
}
impl fmt::Display for RedactedNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the stored secret reference failed validation")
    }
}
impl Error for RedactedNameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

fn parse(text: &str) -> Result<SecretName, DictationDecodeError> {
    // InvalidName owns and prints its input. This central rule preflight
    // prevents invalid/oversized text from entering that diagnostic at all;
    // the existing constructor remains the sole owner of the typed invariant.
    if !NameKind::Secret.accepts(text) {
        return Err(DictationDecodeError::SecretReference { source: None });
    }
    text.parse()
        .map_err(|source| DictationDecodeError::SecretReference {
            source: Some(RedactedNameError::from(source)),
        })
}

/// Reuse the central name rule without allowing its payload into serde errors.
pub(super) fn deserialize<'de, D: Deserializer<'de>>(decoder: D) -> Result<SecretName, D::Error> {
    decoder.deserialize_any(Name)
}

struct Name;
impl Visitor<'_> for Name {
    type Value = SecretName;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the name of a stored secret")
    }
    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        parse(text).map_err(E::custom)
    }
    // Foreign JSON scalars are rejected at this edge without serde formatting
    // their values; even a numeric paste must not enter a diagnostic.
    fn visit_i64<E: de::Error>(self, _value: i64) -> Result<Self::Value, E> {
        Err(wrong_type())
    }
    fn visit_u64<E: de::Error>(self, _value: u64) -> Result<Self::Value, E> {
        Err(wrong_type())
    }
    fn visit_f64<E: de::Error>(self, _value: f64) -> Result<Self::Value, E> {
        Err(wrong_type())
    }
    fn visit_bool<E: de::Error>(self, _value: bool) -> Result<Self::Value, E> {
        Err(wrong_type())
    }
}

fn wrong_type<E: de::Error>() -> E {
    E::custom(DictationDecodeError::SecretReference { source: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pasted_credential_never_enters_errors_or_their_source_chain() {
        let pasted = "synthetic-inline-credential-not-a-name";
        let error = parse(pasted).unwrap_err();
        assert!(!error.to_string().contains(pasted));
        assert!(!format!("{error:?}").contains(pasted));
        let wrapped = RedactedNameError::from(pasted.parse::<SecretName>().unwrap_err());
        assert!(!format!("{wrapped:?}").contains(pasted));
        let source = wrapped.source().unwrap();
        assert!(!source.to_string().contains(pasted));
        assert_eq!(wrapped.0.kind, NameKind::Secret);
        assert_eq!(
            parse("DICTATION_TEST_KEY").unwrap().as_str(),
            "DICTATION_TEST_KEY"
        );
    }
}
