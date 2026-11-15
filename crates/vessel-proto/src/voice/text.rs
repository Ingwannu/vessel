//! Synthesis input is validated and redacted; serializing is its only wire disclosure.

use std::fmt;
use std::sync::Arc;

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Visitor},
};

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
        check(&text)?;
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
        deserializer.deserialize_any(Speech)
    }
}

// The outer JSON decoder still owns its escaped-string scratch buffer. This
// edge avoids a second unbounded owned String and never echoes numeric pastes.
fn check(text: &str) -> Result<(), VoiceDecodeError> {
    if text.len() > MAX_SPEECH_BYTES || crate::workspace::trim_project_text(text).is_empty() {
        return Err(VoiceDecodeError::Speech);
    }
    Ok(())
}
struct Speech;
impl Visitor<'_> for Speech {
    type Value = SpeechText;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded nonblank speech text")
    }
    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        check(text).map_err(E::custom)?;
        Ok(SpeechText(Arc::from(text)))
    }
    fn visit_i64<E: de::Error>(self, _value: i64) -> Result<Self::Value, E> {
        Err(E::custom(VoiceDecodeError::Speech))
    }
    fn visit_u64<E: de::Error>(self, _value: u64) -> Result<Self::Value, E> {
        Err(E::custom(VoiceDecodeError::Speech))
    }
    fn visit_f64<E: de::Error>(self, _value: f64) -> Result<Self::Value, E> {
        Err(E::custom(VoiceDecodeError::Speech))
    }
    fn visit_bool<E: de::Error>(self, _value: bool) -> Result<Self::Value, E> {
        Err(E::custom(VoiceDecodeError::Speech))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speech_scalar_pastes_never_enter_wire_error_diagnostics() {
        for wire in [
            "1234567890123456789",
            "-1234567890123456789",
            "123456789.125",
            "true",
        ] {
            let error = serde_json::from_str::<SpeechText>(wire).unwrap_err();
            assert!(!error.to_string().contains(wire));
            assert!(!format!("{error:?}").contains(wire));
        }
    }
    #[test]
    fn borrowed_escaped_and_owned_speech_share_one_size_and_blank_rule() {
        let plain = "a".repeat(MAX_SPEECH_BYTES);
        let valid = serde_json::to_string(&plain).unwrap();
        assert_eq!(
            serde_json::from_str::<SpeechText>(&valid).unwrap().as_str(),
            plain
        );
        let escaped = r#""line one\nline two \uD55C""#;
        assert_eq!(
            serde_json::from_str::<SpeechText>(escaped)
                .unwrap()
                .as_str(),
            "line one\nline two 한"
        );
        for bad in [
            String::new(),
            " \n\t".to_owned(),
            "a".repeat(MAX_SPEECH_BYTES.checked_add(1).unwrap()),
        ] {
            assert!(SpeechText::try_from(bad.clone()).is_err());
            assert!(
                serde_json::from_str::<SpeechText>(&serde_json::to_string(&bad).unwrap()).is_err()
            );
        }
    }
}
