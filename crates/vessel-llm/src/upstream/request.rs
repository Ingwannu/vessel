//! Shared typed request encoding; policy and catalog lookup stay with the caller.

use super::{anthropic, chat, responses};
use crate::{error::TranslateError, ir};
use bytes::Bytes;

/// One provider's API options, shared by streaming and complete callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Options {
    /// Messages options, preserving the provider's thinking policy.
    Anthropic(anthropic::Options),
    /// Responses options, preserving its sampling policy.
    Responses(responses::Options),
    /// Chat options, preserving its declared compatibility flags.
    Chat(chat::Options),
}

impl Options {
    /// Encode through the existing streaming API without changing its policy.
    ///
    /// # Errors
    /// The existing encoder cannot represent part of the neutral request.
    pub fn streamed(&self, request: &ir::Request) -> Result<Bytes, TranslateError> {
        let bytes = match self {
            Self::Anthropic(options) => serde_json::to_vec(&anthropic::request(request, options)?)?,
            Self::Responses(options) => serde_json::to_vec(&responses::request(request, options)?)?,
            Self::Chat(options) => serde_json::to_vec(&chat::request(request, options)?)?,
        };
        Ok(Bytes::from(bytes))
    }
}

#[cfg(test)]
mod tests;
