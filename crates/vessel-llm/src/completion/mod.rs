//! Strict, bounded tool-free complete replies. Provider grammar stays here;
//! routing, credentials, semantic JSON and publishing belong to the caller.

#[cfg(test)]
mod api_tests;
mod read;
mod request;
pub use request::{OutputTokens, RequestError};
#[cfg(test)]
mod tests;

use crate::{
    client::{Api, Redactor},
    ir,
};
use serde::{Deserialize, de::DeserializeOwned};
use std::{fmt, sync::Arc};

/// The whole-reply byte cap, also used by the caller's HTTP policy.
pub const MAX_COMPLETION_BYTES: usize = 64 * 1024;

// Metadata consumers request one plain answer, never select among candidates.
const COMPLETION_OUTPUTS: usize = 1;

/// A text result whose Debug representation cannot quote provider content.
#[derive(Clone, PartialEq, Eq)]
pub struct CompletionText(Arc<str>);
impl CompletionText {
    /// Borrow for the caller's semantic decoder, never for diagnostic output.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for CompletionText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CompletionText(..)")
    }
}

/// Usage and known endings survive semantic output refusal.
#[derive(Debug)]
pub struct Completion {
    /// Existing neutral accounting; omitted provider counts remain unknown/zero.
    pub usage: ir::Usage,
    /// A known reported stop, without inventing `EndTurn` for unknown reasons.
    pub stop: Option<ir::StopReason>,
    /// Only explicit normal, tool-free replies can expose text.
    pub text: Result<CompletionText, CompletionError>,
}

/// Retains a typed JSON cause without exposing serde's quoted input.
/// Its category is diagnostic; the original stays sealed at this privacy edge.
pub struct JsonFailure(serde_json::Error);
impl JsonFailure {
    /// Preserve a model-content decoder cause while sealing its quoted text.
    #[must_use]
    pub const fn new(cause: serde_json::Error) -> Self {
        Self(cause)
    }
    /// Classify the retained cause without exposing response text.
    #[must_use]
    pub fn category(&self) -> serde_json::error::Category {
        self.0.classify()
    }
}
impl fmt::Debug for JsonFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonFailure")
            .field("category", &self.category())
            .finish_non_exhaustive()
    }
}
impl fmt::Display for JsonFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("model response JSON does not match its API")
    }
}
// A third-party serde error may embed the response in its Display. Retain
// it typed above, but do not reopen that text through a generic error chain.
impl std::error::Error for JsonFailure {}

/// Classified refusals contain no provider text or credential.
#[derive(Debug, thiserror::Error)]
pub enum CompletionError {
    /// Reject before parsing or allocating a reply tree.
    #[error("completion exceeds {MAX_COMPLETION_BYTES} bytes")]
    TooLarge,
    /// JSON is text; the UTF-8 cause contains positions, never body bytes.
    #[error("completion is not UTF-8")]
    NotText(#[source] std::str::Utf8Error),
    /// The cause is sealed because serde can quote untrusted output.
    #[error("completion does not match its API")]
    Decode(#[from] JsonFailure),
    /// A successful HTTP status may still hold an error object.
    #[error("provider declared a failed completion")]
    Failed,
    /// Never repair a missing, unknown or incomplete ending.
    #[error("provider did not explicitly finish a normal turn")]
    NotComplete,
    /// Never choose one of multiple answers or guess an absent result.
    #[error("completion must contain exactly {COMPLETION_OUTPUTS} answer")]
    Outputs,
    /// Tool calls are refused even when the finish reason says stop.
    #[error("completion contains a tool call")]
    Tool,
    /// Refusal text is not an answer to the requested schema.
    #[error("provider refused the completion")]
    Refusal,
    /// Unknown or non-text content cannot establish a plain answer.
    #[error("completion contains unsupported output")]
    Unsupported,
    /// Reasoning alone is not a text result.
    #[error("completion contains no text")]
    Empty,
    /// A provider must not return its own credential.
    #[error("completion quoted a credential")]
    QuotedKey,
}

#[derive(Deserialize)]
struct ErrorHeader {
    #[serde(default)]
    error: Option<vessel_proto::RawJson>,
}

/// Decode an already bounded HTTP body without changing proxy stream policy.
/// Redaction precedes parsing and repeats at the decoded text boundary.
#[must_use]
pub fn decode(api: Api, bytes: &[u8], redactor: &Redactor) -> Completion {
    let result = (|| {
        if bytes.len() > MAX_COMPLETION_BYTES {
            return Err(CompletionError::TooLarge);
        }
        let text = std::str::from_utf8(bytes).map_err(CompletionError::NotText)?;
        if redactor.holds(text) {
            return Err(CompletionError::QuotedKey);
        }
        let failed = json::<ErrorHeader>(text)?.error.is_some();
        let answer = match api {
            Api::AnthropicMessages => read::anthropic(text),
            Api::OpenAiResponses => read::responses(text),
            Api::OpenAiChat => read::chat(text),
        };
        if failed {
            // An error-only body has no accounting. A complete envelope
            // may still report usage; refusing its text must not erase it.
            let mut answer = answer.unwrap_or_else(|_unreadable_error_body| {
                finish(ir::Usage::default(), None, Err(CompletionError::Failed))
            });
            answer.text = Err(CompletionError::Failed);
            Ok(answer)
        } else {
            answer
        }
    })();
    let mut completion =
        result.unwrap_or_else(|error| finish(ir::Usage::default(), None, Err(error)));
    if completion
        .text
        .as_ref()
        .is_ok_and(|text| redactor.holds(text.as_str()))
    {
        completion.text = Err(CompletionError::QuotedKey);
    }
    completion
}

fn json<T: DeserializeOwned>(text: &str) -> Result<T, CompletionError> {
    serde_json::from_str(text).map_err(|cause| JsonFailure::new(cause).into())
}

fn finish(
    usage: ir::Usage,
    stop: Option<ir::StopReason>,
    text: Result<String, CompletionError>,
) -> Completion {
    let text = text.and_then(|text| {
        match stop {
            Some(ir::StopReason::EndTurn) => {}
            Some(ir::StopReason::ToolUse) => return Err(CompletionError::Tool),
            Some(ir::StopReason::Refusal) => return Err(CompletionError::Refusal),
            None
            | Some(
                ir::StopReason::MaxTokens
                | ir::StopReason::StopSequence
                | ir::StopReason::PauseTurn,
            ) => return Err(CompletionError::NotComplete),
        }
        if text.trim().is_empty() {
            return Err(CompletionError::Empty);
        }
        Ok(CompletionText(Arc::from(text)))
    });
    Completion { usage, stop, text }
}

fn only<T>(mut values: Vec<T>) -> Result<T, CompletionError> {
    if values.len() != COMPLETION_OUTPUTS {
        return Err(CompletionError::Outputs);
    }
    values.pop().ok_or(CompletionError::Outputs)
}
