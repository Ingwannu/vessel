//! Validate opaque provider edits before a connector can perform any effect.
use std::{fmt, str::FromStr};
use vessel_proto::RawJson;

/// Provider JSON is validated once and never decoded into an untyped tree.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ModifiedInput(RawJson);

impl fmt::Debug for ModifiedInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ModifiedInput([redacted])")
    }
}

/// Preserve the JSON parser cause while the displayed error stays input-free.
#[derive(Debug, thiserror::Error)]
#[error("--input must contain one valid JSON value")]
pub(crate) struct InvalidInput(#[source] serde_json::Error);

impl FromStr for ModifiedInput {
    type Err = InvalidInput;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        RawJson::from_text(text.to_owned())
            .map(Self)
            .map_err(InvalidInput)
    }
}

impl ModifiedInput {
    /// Opaque passthrough; only the owning provider may inspect its schema.
    pub(crate) const fn raw(&self) -> &RawJson {
        &self.0
    }
}
