//! Translate catalog flags once; API grammar remains in vessel-llm.

use super::config::{Flavor, Provider};
use vessel_llm::upstream::{self, Options};

impl Provider {
    pub(super) fn upstream_options(&self) -> Options {
        let provider = self.name.to_string();
        match &self.flavor {
            Flavor::Anthropic {
                adaptive_thinking,
                default_max_tokens,
            } => Options::Anthropic(upstream::anthropic::Options {
                provider,
                default_max_tokens: *default_max_tokens,
                adaptive_thinking: *adaptive_thinking,
            }),
            Flavor::Responses { sampling } => Options::Responses(upstream::responses::Options {
                provider,
                sampling: *sampling,
            }),
            Flavor::Chat {
                parallel_tool_calls,
                max_tokens_field,
                reasoning,
                replay_reasoning,
            } => Options::Chat(upstream::chat::Options {
                provider,
                parallel_tool_calls: *parallel_tool_calls,
                max_tokens_field: *max_tokens_field,
                reasoning: *reasoning,
                replay_reasoning: *replay_reasoning,
            }),
        }
    }
}
