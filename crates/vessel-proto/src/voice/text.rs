//! Synthesis input is validated and redacted; serializing is its only wire disclosure.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{MAX_SPEECH_BYTES, VoiceDecodeError};

/// Nonblank bounded text handed intentionally to the selected speech provider.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct SpeechText(Arc<str>);

impl SpeechText {
    /// Borrow only at the segmenter and provider boundary, never for diagnostics.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SpeechText {
    type Error = VoiceDecodeError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text.trim().is_empty() || text.len() > MAX_SPEECH_BYTES {
            return Err(VoiceDecodeError::Speech);
        }
        Ok(Self(Arc::from(text)))
    }
}

impl fmt::Debug for SpeechText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpeechText")
            .field("bytes", &self.0.len())
            .finish()
    }
}

impl Serialize for SpeechText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SpeechText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::try_from(text).map_err(serde::de::Error::custom)
    }
}
