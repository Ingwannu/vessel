//! The saved-settings parse boundary, not wire validation or file recovery.
//!
//! Serde may quote a credential supplied as the wrong type. Keep that cause
//! internally, while every public diagnostic and source remains value-free.

use std::fmt;

use serde::de::DeserializeOwned;
use serde_path_to_error::Segment;

/// Bound diagnostic allocation independently of the foreign path's depth.
const MAX_PATH_SEGMENTS: usize = 16;

/// Safe metadata for a saved-settings refusal, with its serde cause private.
///
/// The cause is intentionally not an `Error::source`: arbitrary error-chain
/// renderers must never recover values that serde quotes. Recovery retains the
/// original file; diagnostics retain only its category and masked field path.
pub struct SettingsDecodeFailure {
    cause: serde_json::Error,
    path: FieldPath,
}

struct FieldPath(Box<str>);

impl FieldPath {
    fn masked(path: &serde_path_to_error::Path) -> Self {
        let parts: String = path
            .iter()
            .take(MAX_PATH_SEGMENTS)
            .map(|segment| match segment {
                Segment::Seq { index } => format!("[{index}]"),
                // Even a struct-looking key can come from a person's map.
                Segment::Map { .. } | Segment::Enum { .. } => ".<field>".to_owned(),
                Segment::Unknown => ".<?>".to_owned(),
            })
            .collect();
        let suffix = if path.iter().len() > MAX_PATH_SEGMENTS {
            ".…"
        } else {
            ""
        };
        Self(format!("${parts}{suffix}").into_boxed_str())
    }

    fn root() -> Self {
        Self("$".into())
    }
}

impl fmt::Display for SettingsDecodeFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.cause.classify() {
            serde_json::error::Category::Io => "I/O error",
            serde_json::error::Category::Syntax => "invalid JSON",
            serde_json::error::Category::Data => "invalid field",
            serde_json::error::Category::Eof => "incomplete JSON",
        };
        write!(out, "{kind} at {}", self.path.0)
    }
}

impl fmt::Debug for SettingsDecodeFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, out)
    }
}

impl std::error::Error for SettingsDecodeFailure {}

pub(super) fn settings<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, SettingsDecodeFailure> {
    let mut reader = serde_json::Deserializer::from_slice(bytes);
    let value = serde_path_to_error::deserialize(&mut reader).map_err(|error| {
        let path = FieldPath::masked(error.path());
        SettingsDecodeFailure {
            cause: error.into_inner(),
            path,
        }
    })?;
    // Preserve from_slice's single-value rule, including trailing whitespace.
    reader.end().map_err(|cause| SettingsDecodeFailure {
        cause,
        path: FieldPath::root(),
    })?;
    Ok(value)
}

/// Encoding failure also stays behind the value-free diagnostic boundary.
pub(super) fn encoding(cause: serde_json::Error) -> SettingsDecodeFailure {
    SettingsDecodeFailure {
        cause,
        path: FieldPath::root(),
    }
}

#[cfg(test)]
mod tests;
