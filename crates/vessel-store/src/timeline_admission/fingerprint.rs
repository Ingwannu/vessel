//! Encoding edge for canonical receipt hashing. A JSON tree is needed only
//! here to sort nested opaque argument objects; no tree leaves this module.

use std::{collections::BTreeMap, fmt};

use serde_json::Value;
use sha2::{Digest, Sha256};
use vessel_proto::{Creator, timeline::SubmitPrompt};

use crate::StoreError;

/// SHA-256 has one fixed wire/storage width, kept separate from content lengths.
const FINGERPRINT_BYTES: usize = 32;

/// A fingerprint never exposes prompt content or its digest in diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ReceiptFingerprint([u8; FINGERPRINT_BYTES]);

impl fmt::Debug for ReceiptFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReceiptFingerprint([redacted])")
    }
}

impl ReceiptFingerprint {
    pub(super) fn for_prompt(prompt: &SubmitPrompt, creator: &Creator) -> Result<Self, StoreError> {
        let value = serde_json::to_value((prompt, creator)).map_err(StoreError::Encode)?;
        let encoded = serde_json::to_vec(&sorted(value)).map_err(StoreError::Encode)?;
        Ok(Self(Sha256::digest(encoded).into()))
    }

    pub(super) const fn bytes(&self) -> &[u8; FINGERPRINT_BYTES] {
        &self.0
    }

    pub(super) fn from_sql(bytes: Vec<u8>) -> Result<Self, StoreError> {
        bytes
            .try_into()
            .map(Self)
            .map_err(|_wrong_length| StoreError::AdmissionIntegrity)
    }
}

fn sorted(value: Value) -> Value {
    match value {
        Value::Object(values) => {
            let ordered: BTreeMap<_, _> = values
                .into_iter()
                .map(|(key, value)| (key, sorted(value)))
                .collect();
            Value::Object(ordered.into_iter().collect())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(sorted).collect()),
        other @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => other,
    }
}
