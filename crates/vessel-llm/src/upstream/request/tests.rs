//! Byte parity with the individual encoders; this entry point adds no policy.

use super::*;
use crate::{fixtures, front};

fn choices() -> Vec<Options> {
    let mut choices = vec![
        Options::Responses(responses::Options {
            provider: "fixture-provider".into(),
            sampling: false,
        }),
        Options::Responses(responses::Options {
            provider: "fixture-provider".into(),
            sampling: true,
        }),
    ];
    for adaptive_thinking in [false, true] {
        choices.push(Options::Anthropic(anthropic::Options {
            provider: "fixture-provider".into(),
            default_max_tokens: 4096,
            adaptive_thinking,
        }));
    }
    for reasoning in [
        chat::ReasoningStyle::None,
        chat::ReasoningStyle::Effort,
        chat::ReasoningStyle::OpenRouter,
    ] {
        for max_tokens_field in [
            chat::MaxTokensField::MaxTokens,
            chat::MaxTokensField::MaxCompletionTokens,
        ] {
            for parallel_tool_calls in [false, true] {
                for replay_reasoning in [false, true] {
                    choices.push(Options::Chat(chat::Options {
                        provider: "fixture-provider".into(),
                        parallel_tool_calls,
                        max_tokens_field,
                        reasoning,
                        replay_reasoning,
                    }));
                }
            }
        }
    }
    choices
}

#[test]
fn shared_options_preserve_each_existing_encoders_bytes() {
    let requests = [
        front::anthropic::request(serde_json::from_str(fixtures::ANTHROPIC_REQUEST).unwrap())
            .unwrap()
            .0,
        front::responses::request(serde_json::from_str(fixtures::RESPONSES_REQUEST).unwrap())
            .unwrap()
            .0,
    ];
    for request in &requests {
        for options in choices() {
            let direct = match &options {
                Options::Anthropic(options) => {
                    serde_json::to_vec(&anthropic::request(request, options).unwrap()).unwrap()
                }
                Options::Responses(options) => {
                    serde_json::to_vec(&responses::request(request, options).unwrap()).unwrap()
                }
                Options::Chat(options) => {
                    serde_json::to_vec(&chat::request(request, options).unwrap()).unwrap()
                }
            };
            assert_eq!(options.streamed(request).unwrap(), Bytes::from(direct));
        }
    }
}
