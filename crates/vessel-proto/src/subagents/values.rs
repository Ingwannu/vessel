//! Provider strings are parsed once before they become identities or UI text.

use serde::{Deserialize, Deserializer, Serialize};
use std::{str::FromStr, sync::Arc};

/// Refusals are structural, never an echo of untrusted provider text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SubagentValueError {
    /// A provider child needs an actual opaque identity.
    #[error("provider child id is empty")]
    EmptyId,
    /// Bounds apply to bytes so the wire and retained state share a budget.
    #[error("provider child id exceeds {} bytes", ProviderChildId::MAX_BYTES)]
    IdTooLong,
    /// Control characters cannot be part of a key.
    #[error("provider child id contains a control character")]
    IdControl,
    /// Descriptive metadata is not an unbounded transcript container.
    #[error("provider child text exceeds {} bytes", ProviderText::MAX_BYTES)]
    TextTooLong,
    /// Multiline text is allowed; terminal control bytes are not.
    #[error("provider child text contains an unsupported control character")]
    TextControl,
}

/// Opaque provider-owned id, scoped by the managed parent; never a `SessionId`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ProviderChildId(Arc<str>);

impl ProviderChildId {
    /// A security floor absent from Paseo's unbounded descriptor ids.
    pub const MAX_BYTES: usize = 512;
    /// Preserve the provider's identity without case folding or interpretation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for ProviderChildId {
    type Err = SubagentValueError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() {
            return Err(SubagentValueError::EmptyId);
        }
        if text.len() > Self::MAX_BYTES {
            return Err(SubagentValueError::IdTooLong);
        }
        if text.chars().any(char::is_control) {
            return Err(SubagentValueError::IdControl);
        }
        Ok(Self(Arc::from(text)))
    }
}
impl<'de> Deserialize<'de> for ProviderChildId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Plain provider metadata; rendering never interprets it as configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ProviderText(Arc<str>);
impl ProviderText {
    /// Explicit retained-metadata budget; full child output belongs in events.
    pub const MAX_BYTES: usize = 64 * 1024;
    /// Borrow for ordinary text rendering or projection comparisons.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for ProviderText {
    type Err = SubagentValueError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > Self::MAX_BYTES {
            return Err(SubagentValueError::TextTooLong);
        }
        if text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err(SubagentValueError::TextControl);
        }
        Ok(Self(Arc::from(text)))
    }
}
impl<'de> Deserialize<'de> for ProviderText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_keys_preserve_spelling_and_refuse_invalid_or_overlong_values() {
        assert_eq!(
            "thread/한글:A".parse::<ProviderChildId>().unwrap().as_str(),
            "thread/한글:A"
        );
        assert_eq!(
            "".parse::<ProviderChildId>(),
            Err(SubagentValueError::EmptyId)
        );
        assert_eq!(
            "bad\nkey".parse::<ProviderChildId>(),
            Err(SubagentValueError::IdControl)
        );
        assert!(
            "x".repeat(ProviderChildId::MAX_BYTES)
                .parse::<ProviderChildId>()
                .is_ok()
        );
        assert_eq!(
            "x".repeat(ProviderChildId::MAX_BYTES.saturating_add(1))
                .parse::<ProviderChildId>(),
            Err(SubagentValueError::IdTooLong)
        );
    }
    #[test]
    fn provider_metadata_accepts_multiline_text_but_refuses_control_bytes_and_overflow() {
        assert!(
            "작업 설명\nsecond line\tcontext"
                .parse::<ProviderText>()
                .is_ok()
        );
        assert!(
            "x".repeat(ProviderText::MAX_BYTES)
                .parse::<ProviderText>()
                .is_ok()
        );
        assert_eq!(
            "x".repeat(ProviderText::MAX_BYTES.saturating_add(1))
                .parse::<ProviderText>(),
            Err(SubagentValueError::TextTooLong)
        );
        assert_eq!(
            "escape\u{001b}[0m".parse::<ProviderText>(),
            Err(SubagentValueError::TextControl)
        );
    }
    #[test]
    fn wire_decoding_cannot_bypass_the_string_bounds() {
        let raw = serde_json::to_string(&"x".repeat(ProviderChildId::MAX_BYTES.saturating_add(1)))
            .unwrap();
        assert!(serde_json::from_str::<ProviderChildId>(&raw).is_err());
        let raw =
            serde_json::to_string(&"x".repeat(ProviderText::MAX_BYTES.saturating_add(1))).unwrap();
        assert!(serde_json::from_str::<ProviderText>(&raw).is_err());
    }
}
