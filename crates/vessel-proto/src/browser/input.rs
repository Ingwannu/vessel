//! Page input is semantic text, never a diagnostic string or host command.
//!
//! These types deliberately redact Debug because browser typing arguments can
//! contain credentials. Their Serialize implementation is the intentional
//! transport copy; the harness's raw tool event may retain a separate copy.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Form fill/type can clear a field, so empty text is valid.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct InputText(Arc<str>);

impl InputText {
    /// The UTF-8 string is already decoded at the transport boundary.
    #[must_use]
    pub fn new(text: impl Into<Arc<str>>) -> Self {
        Self(text.into())
    }

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

/// A missing predicate/function/key is a typed refusal, without the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("browser argument must not be empty")]
pub struct EmptyBrowserInput;

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
    type Error = EmptyBrowserInput;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(EmptyBrowserInput)
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
        let input = InputText::new(secret);
        assert!(!format!("{input:?}").contains(secret));
        assert_eq!(
            serde_json::to_string(&input).unwrap(),
            format!("\"{secret}\"")
        );
        assert!(serde_json::from_str::<RequiredInput>(r#""""#).is_err());
        assert!(serde_json::from_str::<InputText>(r#""""#).is_ok());
    }

    #[test]
    fn a_wait_fragment_need_not_be_a_complete_url() {
        let wait: WaitFor =
            serde_json::from_str(r#"{"kind":"url","value":"/ready#done"}"#).unwrap();
        assert!(matches!(wait, WaitFor::Url(_)));
        assert!(serde_json::from_str::<WaitFor>(r#"{"kind":"url","value":""}"#).is_err());
    }
}
