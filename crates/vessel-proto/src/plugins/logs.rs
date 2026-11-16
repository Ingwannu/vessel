//! Bounded plugin diagnostics; only the authorized management path exposes text.

use std::{fmt, str::FromStr, sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Paseo plugins/runtime.ts:43, retained across reload/disable/failure.
pub const MAX_LOG_ENTRIES: usize = 500;
/// Paseo plugins/runtime.ts:44, measured as encoded UTF-8 message bytes.
pub const MAX_LOG_BYTES: usize = 256 * 1024;
/// Paseo plugins/runtime.ts:45, one diagnostic prefix.
pub const MAX_LOG_LINE_BYTES: usize = 16 * 1024;

/// Diagnostics are display data, not a command or an error message safe for
/// scoped clients. Ambient tracing must log metadata only, never this value.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct LogText(Arc<str>);

impl fmt::Debug for LogText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LogText(<private diagnostic>)")
    }
}

impl LogText {
    /// The authorized logs screen may display this; no general error path may.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Retain a UTF-8-safe prefix and mark truncation explicitly. Native bytes
    /// may be invalid UTF-8; replacement happens before enforcing the final cap.
    #[must_use]
    pub fn capture(bytes: &[u8]) -> (Self, LogTruncation) {
        let bounded = bytes.get(..MAX_LOG_LINE_BYTES).unwrap_or(bytes);
        let text = String::from_utf8_lossy(bounded);
        let mut remaining = MAX_LOG_LINE_BYTES;
        let prefix: String = text
            .chars()
            .take_while(|character| {
                if let Some(next) = remaining.checked_sub(character.len_utf8()) {
                    remaining = next;
                    true
                } else {
                    false
                }
            })
            .collect();
        let truncation = if bytes.len() > bounded.len() || prefix.len() < text.len() {
            LogTruncation::PrefixRetained
        } else {
            LogTruncation::Complete
        };
        (Self(Arc::from(prefix)), truncation)
    }
}

/// Serialized diagnostics are already bounded; reject corrupt stored/wire text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("plugin diagnostic text exceeds {MAX_LOG_LINE_BYTES} UTF-8 bytes")]
pub struct InvalidLogText;

impl FromStr for LogText {
    type Err = InvalidLogText;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > MAX_LOG_LINE_BYTES {
            return Err(InvalidLogText);
        }
        Ok(Self(Arc::from(text)))
    }
}

impl fmt::Display for LogText {
    // Display must not leak a helper's arbitrary output into a CLI error chain.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("private plugin diagnostic")
    }
}

impl Serialize for LogText {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for LogText {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// An overlong line is explicit UI metadata rather than a silently shortened result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LogTruncation {
    /// The complete line fit.
    Complete,
    /// The diagnostic prefix fit; the rest is intentionally discarded.
    PrefixRetained,
}

/// Keep stream identity rather than parsing a string in a renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    /// Trusted host lifecycle messages; helper stdout is the framing channel.
    Host,
    /// Arbitrary helper diagnostics, restricted to management readers.
    Stderr,
}

/// Per-installation increasing diagnostic identity; never wraps into an older row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct LogSequence(u64);

impl LogSequence {
    /// Allocate at the store's checked counter boundary.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Numeric ordering for the management-only log tail.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Diagnostic content is never part of a scoped catalog/list failure response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LogEntry {
    /// Stable row identity within a node lifetime.
    pub sequence: LogSequence,
    /// UTC receipt time, not helper-supplied log clock.
    pub at: DateTime<Utc>,
    /// Core lifecycle or private helper stderr.
    pub stream: LogStream,
    /// Management-only text; Debug and Display are deliberately redacted.
    pub message: LogText,
    /// Explicit evidence that an overlong line lost its suffix.
    pub truncation: LogTruncation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_bounded_and_do_not_escape_debug_or_display() {
        let bytes = vec![0xff; MAX_LOG_LINE_BYTES.saturating_add(1)];
        let (text, truncated) = LogText::capture(&bytes);
        assert!(text.as_str().len() <= MAX_LOG_LINE_BYTES);
        assert_eq!(truncated, LogTruncation::PrefixRetained);
        let (text, truncated) = LogText::capture(b"private fixture value");
        assert_eq!(truncated, LogTruncation::Complete);
        assert!(!format!("{text:?} {text}").contains("fixture value"));
        assert_eq!(
            serde_json::from_str::<LogText>(&serde_json::to_string(&text).expect("encode"))
                .expect("decode"),
            text
        );
    }

    #[test]
    fn wire_diagnostics_refuse_excess_instead_of_silently_truncating() {
        let text = "x".repeat(MAX_LOG_LINE_BYTES.saturating_add(1));
        assert!(text.parse::<LogText>().is_err());
        assert!(
            serde_json::from_str::<LogText>(&serde_json::to_string(&text).expect("encode"))
                .is_err()
        );
    }
}
