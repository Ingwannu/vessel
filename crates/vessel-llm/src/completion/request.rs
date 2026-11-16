//! Complete request policy; the existing upstream encoders retain API grammar.

use crate::{
    error::TranslateError,
    ir,
    upstream::{self, Options},
};
use bytes::Bytes;
use std::{fmt, num::NonZeroU32};

/// A positive completion output budget, never an absent or unlimited cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputTokens(NonZeroU32);

impl OutputTokens {
    /// Require positivity at construction so preparation always has a cap.
    #[must_use]
    pub const fn new(tokens: NonZeroU32) -> Self {
        Self(tokens)
    }

    /// The encoded provider field uses the same token count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for OutputTokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A complete request is refused before its bytes can reach the HTTP client.
#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    /// Keep the existing typed encoder cause.
    #[error("cannot encode complete request")]
    Translate(#[from] TranslateError),
    /// The encoder must not silently raise the caller's budget.
    #[error("provider encoding exceeds completion budget of {limit} tokens")]
    Budget {
        /// The caller's positive budget, used in the diagnostic too.
        limit: OutputTokens,
    },
}

impl Options {
    /// Encode one whole answer with a checked cap and the metadata effort policy.
    /// The owned request permits normalization without a second history clone.
    ///
    /// # Errors
    /// Encoding failed, or the provider's thinking policy raised the output cap.
    pub fn complete(
        &self,
        mut request: ir::Request,
        limit: OutputTokens,
    ) -> Result<Bytes, RequestError> {
        request.max_output_tokens = Some(limit.get());
        request.reasoning = self.complete_reasoning(request.reasoning);
        let bytes = match self {
            Self::Anthropic(options) => {
                let mut body = upstream::anthropic::request(&request, options)?;
                check_budget(body.max_tokens, limit)?;
                body.stream = Some(false);
                serde_json::to_vec(&body).map_err(TranslateError::from)?
            }
            Self::Responses(options) => {
                let mut body = upstream::responses::request(&request, options)?;
                check_budget(body.max_output_tokens, limit)?;
                body.stream = Some(false);
                serde_json::to_vec(&body).map_err(TranslateError::from)?
            }
            Self::Chat(options) => {
                let mut body = upstream::chat::request(&request, options)?;
                check_budget(body.max_tokens.or(body.max_completion_tokens), limit)?;
                body.stream = Some(false);
                body.stream_options = None;
                serde_json::to_vec(&body).map_err(TranslateError::from)?
            }
        };
        Ok(Bytes::from(bytes))
    }

    fn complete_reasoning(&self, reasoning: ir::Reasoning) -> ir::Reasoning {
        match self {
            Self::Anthropic(_) if reasoning == ir::Reasoning::Effort(ir::Effort::Minimal) => {
                ir::Reasoning::Default
            }
            Self::Chat(options)
                if options.reasoning == upstream::chat::ReasoningStyle::None
                    || reasoning == ir::Reasoning::Effort(ir::Effort::Max) =>
            {
                ir::Reasoning::Default
            }
            Self::Anthropic(_) | Self::Responses(_) | Self::Chat(_) => reasoning,
        }
    }
}

fn check_budget(encoded: Option<u32>, limit: OutputTokens) -> Result<(), RequestError> {
    if encoded.is_some_and(|tokens| tokens <= limit.get()) {
        Ok(())
    } else {
        Err(RequestError::Budget { limit })
    }
}

#[cfg(test)]
mod tests;
