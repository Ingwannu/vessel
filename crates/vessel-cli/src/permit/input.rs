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

/// The standard typed parser keeps its original error source, but its printable
/// argument context must not echo an opaque provider payload on invalid JSON.
#[derive(Clone)]
pub(crate) struct Parser;
impl clap::builder::TypedValueParser for Parser {
    type Value = ModifiedInput;
    fn parse_ref(
        &self,
        command: &clap::Command,
        argument: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        use clap::builder::TypedValueParser as _;
        use clap::error::{ContextKind, ContextValue};
        clap::builder::StringValueParser::new()
            .try_map(|text: String| text.parse::<ModifiedInput>())
            .parse_ref(command, argument, value)
            .map_err(|mut error| {
                let _private = error.insert(
                    ContextKind::InvalidValue,
                    ContextValue::String("[redacted]".to_owned()),
                );
                error
            })
    }
}
