//! Encoded fields, not neutral intent, establish the whole-request cost bound.

use super::*;
use crate::wire;

const LIMIT: OutputTokens = OutputTokens::new(NonZeroU32::new(512).unwrap());

fn request(reasoning: ir::Reasoning) -> ir::Request {
    ir::Request {
        model: "fixture-model".into(),
        system: vec!["Respond with JSON.".into()],
        messages: vec![ir::Message {
            role: ir::Role::User,
            parts: vec![ir::Part::Text("Fixture input".into())],
        }],
        tools: Vec::new(),
        tool_choice: ir::ToolChoice::None,
        parallel_tool_calls: None,
        max_output_tokens: None,
        sampling: ir::Sampling::default(),
        stop_sequences: Vec::new(),
        reasoning,
        cache_key: None,
    }
}

fn anthropic(adaptive_thinking: bool) -> Options {
    Options::Anthropic(upstream::anthropic::Options {
        provider: "fixture-provider".into(),
        default_max_tokens: LIMIT.get(),
        adaptive_thinking,
    })
}

fn responses() -> Options {
    Options::Responses(upstream::responses::Options {
        provider: "fixture-provider".into(),
        sampling: false,
    })
}

fn chat(
    max_tokens_field: upstream::chat::MaxTokensField,
    reasoning: upstream::chat::ReasoningStyle,
) -> Options {
    Options::Chat(upstream::chat::Options {
        provider: "fixture-provider".into(),
        parallel_tool_calls: false,
        max_tokens_field,
        reasoning,
        replay_reasoning: false,
    })
}

#[test]
fn all_apis_encode_whole_answers_with_the_callers_cap() {
    for adaptive in [false, true] {
        let body: wire::anthropic::Request = serde_json::from_slice(
            &anthropic(adaptive)
                .complete(request(ir::Reasoning::Default), LIMIT)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body.stream, Some(false));
        assert_eq!(body.max_tokens, Some(LIMIT.get()));
        assert!(body.thinking.is_none());
    }
    let body: wire::responses::Request = serde_json::from_slice(
        &responses()
            .complete(request(ir::Reasoning::Default), LIMIT)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body.stream, Some(false));
    assert_eq!(body.store, Some(false));
    assert_eq!(body.max_output_tokens, Some(LIMIT.get()));
    for field in [
        upstream::chat::MaxTokensField::MaxTokens,
        upstream::chat::MaxTokensField::MaxCompletionTokens,
    ] {
        let body: wire::chat::Request = serde_json::from_slice(
            &chat(field, upstream::chat::ReasoningStyle::Effort)
                .complete(request(ir::Reasoning::Default), LIMIT)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body.stream, Some(false));
        assert!(body.stream_options.is_none());
        match field {
            upstream::chat::MaxTokensField::MaxTokens => {
                assert_eq!(body.max_tokens, Some(LIMIT.get()));
                assert!(body.max_completion_tokens.is_none());
            }
            upstream::chat::MaxTokensField::MaxCompletionTokens => {
                assert_eq!(body.max_completion_tokens, Some(LIMIT.get()));
                assert!(body.max_tokens.is_none());
            }
        }
    }
}

#[test]
fn actual_thinking_increase_is_refused_before_bytes_are_exposed() {
    let request = request(ir::Reasoning::Effort(ir::Effort::Low));
    let error = anthropic(false)
        .complete(request.clone(), LIMIT)
        .unwrap_err();
    assert!(matches!(error, RequestError::Budget { limit } if limit == LIMIT));
    assert!(error.to_string().contains(&LIMIT.to_string()));
    let body: wire::anthropic::Request =
        serde_json::from_slice(&anthropic(true).complete(request, LIMIT).unwrap()).unwrap();
    assert_eq!(body.max_tokens, Some(LIMIT.get()));
    assert!(matches!(
        body.thinking,
        Some(wire::anthropic::Thinking::Adaptive(_))
    ));
    assert_eq!(
        body.output_config.unwrap().effort,
        Some(wire::anthropic::Effort::Low)
    );
}

#[test]
fn unsupported_metadata_efforts_leave_the_provider_default() {
    for adaptive in [false, true] {
        let body: wire::anthropic::Request = serde_json::from_slice(
            &anthropic(adaptive)
                .complete(request(ir::Reasoning::Effort(ir::Effort::Minimal)), LIMIT)
                .unwrap(),
        )
        .unwrap();
        assert!(body.thinking.is_none());
        assert!(body.output_config.is_none());
    }
    for style in [
        upstream::chat::ReasoningStyle::None,
        upstream::chat::ReasoningStyle::Effort,
        upstream::chat::ReasoningStyle::OpenRouter,
    ] {
        let options = chat(upstream::chat::MaxTokensField::MaxCompletionTokens, style);
        let body: wire::chat::Request = serde_json::from_slice(
            &options
                .complete(request(ir::Reasoning::Effort(ir::Effort::Max)), LIMIT)
                .unwrap(),
        )
        .unwrap();
        assert!(body.reasoning_effort.is_none());
        assert!(body.reasoning.is_none());
    }
    let options = chat(
        upstream::chat::MaxTokensField::MaxTokens,
        upstream::chat::ReasoningStyle::None,
    );
    let body: wire::chat::Request = serde_json::from_slice(
        &options
            .complete(request(ir::Reasoning::Effort(ir::Effort::Low)), LIMIT)
            .unwrap(),
    )
    .unwrap();
    assert!(body.reasoning_effort.is_none());
    assert!(body.reasoning.is_none());
}

#[test]
fn represented_efforts_keep_the_requested_level() {
    let body: wire::responses::Request = serde_json::from_slice(
        &responses()
            .complete(request(ir::Reasoning::Effort(ir::Effort::Max)), LIMIT)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        body.reasoning.unwrap().effort,
        Some(wire::responses::ReasoningEffort::ExtraHigh)
    );
    let options = chat(
        upstream::chat::MaxTokensField::MaxCompletionTokens,
        upstream::chat::ReasoningStyle::Effort,
    );
    let body: wire::chat::Request = serde_json::from_slice(
        &options
            .complete(request(ir::Reasoning::Effort(ir::Effort::Low)), LIMIT)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        body.reasoning_effort,
        Some(wire::chat::ReasoningEffort::Low)
    );
}

#[test]
fn a_missing_or_raised_encoded_cap_cannot_be_sent() {
    for cap in [None, Some(LIMIT.get().saturating_add(1))] {
        assert!(matches!(
            check_budget(cap, LIMIT),
            Err(RequestError::Budget { .. })
        ));
    }
    check_budget(Some(LIMIT.get()), LIMIT).unwrap();
    check_budget(Some(NonZeroU32::MIN.get()), LIMIT).unwrap();
}
