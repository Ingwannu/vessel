//! Neutral call accounting shared by proxy and metadata, without recording or I/O.

use vessel_llm::ir;
use vessel_proto::{ModelFailure, ModelStop, TokenCount, TokenUsage};

/// Reuse the proxy's safe failure classification without provider messages.
pub(super) const fn failure_class(class: ir::FailureClass) -> ModelFailure {
    match class {
        ir::FailureClass::RateLimited => ModelFailure::RateLimited,
        ir::FailureClass::Overloaded => ModelFailure::Overloaded,
        ir::FailureClass::UpstreamError => ModelFailure::UpstreamError,
        ir::FailureClass::RefusedRequest => ModelFailure::RefusedRequest,
        ir::FailureClass::ContextTooLong => ModelFailure::ContextTooLong,
        ir::FailureClass::Timeout => ModelFailure::Timeout,
        ir::FailureClass::Truncated => ModelFailure::Truncated,
        ir::FailureClass::Unauthorized => ModelFailure::Unauthorized,
    }
}

/// Preserve a known provider stop when recording either kind of call.
pub(super) const fn stop_of(stop: ir::StopReason) -> ModelStop {
    match stop {
        ir::StopReason::EndTurn => ModelStop::EndTurn,
        ir::StopReason::ToolUse => ModelStop::ToolUse,
        ir::StopReason::MaxTokens => ModelStop::MaxTokens,
        ir::StopReason::StopSequence => ModelStop::StopSequence,
        ir::StopReason::Refusal => ModelStop::Refusal,
        ir::StopReason::PauseTurn => ModelStop::Pause,
    }
}

/// Both call paths keep the existing token-count convention.
pub(super) const fn usage(usage: &ir::Usage) -> TokenUsage {
    TokenUsage {
        input: TokenCount::new(usage.input),
        cache_read: TokenCount::new(usage.cache_read),
        cache_write: TokenCount::new(usage.cache_write),
        output: TokenCount::new(usage.output),
        reasoning: TokenCount::new(usage.reasoning),
    }
}
