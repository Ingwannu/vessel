//! Known provider fields only; malformed score metadata preserves unspecified confidence.
use serde::{Deserialize, Deserializer, de::IgnoredAny};
use std::borrow::Cow;

#[derive(Deserialize)]
pub(super) struct Response<'a> {
    #[serde(borrow)]
    pub(super) text: Cow<'a, str>,
    #[serde(default)]
    pub(super) logprobs: Tolerant<Vec<Token>>,
}

// Untagged decoding retains only the known shape or a payload-free marker.
// Foreign parse errors are destroyed by the caller, never put in diagnostics.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Tolerant<T> {
    Known(T),
    Other(IgnoredAny),
}
impl<T> Default for Tolerant<T> {
    fn default() -> Self {
        Self::Other(IgnoredAny)
    }
}
impl<T> Tolerant<T> {
    pub(super) const fn known(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Other(_ignored) => None,
        }
    }
}

#[derive(Deserialize)]
pub(super) struct Token {
    #[serde(rename = "token")]
    _token: String,
    pub(super) logprob: f64,
    #[serde(rename = "bytes", default = "absent_bytes", deserialize_with = "bytes")]
    pub(super) bytes_valid: bool,
}
const fn absent_bytes() -> bool {
    true
}
fn bytes<'de, D: Deserializer<'de>>(decoder: D) -> Result<bool, D::Error> {
    // Missing bytes are valid; a present null or non-number array is malformed.
    Ok(Tolerant::<Vec<f64>>::deserialize(decoder)?
        .known()
        .is_some())
}
