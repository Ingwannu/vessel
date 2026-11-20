//! Page input is semantic text, never a diagnostic string or host command.
//!
//! These types deliberately redact Debug because browser typing arguments can
//! contain credentials. Their Serialize implementation is the intentional
//! transport copy; the harness's raw tool event may retain a separate copy.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Bound one command before it can occupy a target queue slot.
pub const MAX_BROWSER_INPUT_BYTES: usize = 128 * 1024;

/// Form fill/type can clear a field, so empty text is valid.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct InputText(Arc<str>);

impl InputText {
    /// Only an input/CDP serializer should expose the actual text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for InputText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InputText([redacted])")
    }
}

/// Input errors never include the supplied text, function or key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidBrowserInput {
    /// A required predicate/function/key cannot be empty.
    #[error("browser argument must not be empty")]
    Empty,
    /// Retained command memory has a fixed per-text ceiling.
    #[error("browser input exceeds {MAX_BROWSER_INPUT_BYTES} UTF-8 bytes")]
    TooLong,
}

impl TryFrom<String> for InputText {
    type Error = InvalidBrowserInput;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_BROWSER_INPUT_BYTES {
            Err(InvalidBrowserInput::TooLong)
        } else {
            Ok(Self(Arc::from(value)))
        }
    }
}
impl From<InputText> for String {
    fn from(value: InputText) -> Self {
        value.0.to_string()
    }
}

/// Wait substrings, page functions and keys cannot be absent or empty.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct RequiredInput(Arc<str>);

impl RequiredInput {
    /// The engine interprets this only in the command's known context.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RequiredInput {
    type Error = InvalidBrowserInput;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(InvalidBrowserInput::Empty)
        } else if value.len() > MAX_BROWSER_INPUT_BYTES {
            Err(InvalidBrowserInput::TooLong)
        } else {
            Ok(Self(Arc::from(value)))
        }
    }
}

impl From<RequiredInput> for String {
    fn from(value: RequiredInput) -> Self {
        value.0.to_string()
    }
}

impl fmt::Debug for RequiredInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RequiredInput([redacted])")
    }
}

/// Wait URL is a substring, not a navigable URL (Paseo service.ts:1005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum WaitFor {
    /// Match the visible page text.
    Text(RequiredInput),
    /// Match a fragment of the current location, without opening a URL.
    Url(RequiredInput),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_copies_are_explicit_and_diagnostics_are_redacted() {
        let secret = "typing-canary-secret";
        let input = InputText::try_from(secret.to_owned()).unwrap();
        assert!(!format!("{input:?}").contains(secret));
        assert_eq!(
            serde_json::to_string(&input).unwrap(),
            format!("\"{secret}\"")
        );
        assert!(serde_json::from_str::<RequiredInput>(r#""""#).is_err());
        assert!(serde_json::from_str::<InputText>(r#""""#).is_ok());
        let oversized = "x".repeat(MAX_BROWSER_INPUT_BYTES.saturating_add(1));
        assert!(InputText::try_from(oversized.clone()).is_err());
        assert!(RequiredInput::try_from(oversized).is_err());
    }

    #[test]
    fn a_wait_fragment_need_not_be_a_complete_url() {
        let wait: WaitFor =
            serde_json::from_str(r#"{"kind":"url","value":"/ready#done"}"#).unwrap();
        assert!(matches!(wait, WaitFor::Url(_)));
        assert!(serde_json::from_str::<WaitFor>(r#"{"kind":"url","value":""}"#).is_err());
    }
}
