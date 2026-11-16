//! Opaque bytes on the wire, as standard base64 text: terminal I/O and
//! fleet blobs share this one encoding (use with `#[serde(with = ...)]`).

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use bytes::Bytes;
use serde::de::{self, Visitor};
use serde::{Deserializer, Serializer};

pub(crate) fn serialize<S: Serializer>(bytes: &Bytes, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(bytes))
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Bytes, D::Error> {
    deserializer.deserialize_str(Base64Visitor {
        decoded_limit: None,
    })
}

/// Bound audio before allocating its decoded payload; existing opaque bytes stay unchanged.
pub(crate) fn deserialize_bounded<'de, D: Deserializer<'de>, const MAX: usize>(
    deserializer: D,
) -> Result<Bytes, D::Error> {
    deserializer.deserialize_str(Base64Visitor {
        decoded_limit: Some(MAX),
    })
}

struct Base64Visitor {
    decoded_limit: Option<usize>,
}

impl Visitor<'_> for Base64Visitor {
    type Value = Bytes;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("base64-encoded bytes")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        if let Some(limit) = self.decoded_limit {
            // Encoded ceiling includes padding; never decode an attacker-sized allocation.
            let encoded_limit = limit.div_ceil(3).saturating_mul(4);
            if text.len() > encoded_limit {
                return Err(E::custom("base64 payload exceeds its byte limit"));
            }
        }
        let bytes = STANDARD.decode(text).map(Bytes::from).map_err(|error| {
            if self.decoded_limit.is_some() {
                // The audio boundary reports its rule, never an offending
                // encoded byte/offset from a private microphone payload.
                E::custom("base64 audio payload is invalid")
            } else {
                E::custom(error)
            }
        })?;
        if self.decoded_limit.is_some_and(|limit| bytes.len() > limit) {
            return Err(E::custom("base64 payload exceeds its byte limit"));
        }
        Ok(bytes)
    }
}
