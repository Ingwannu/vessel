//! Synthetic whole API fixtures: no captured provider content or network.

use super::*;
use crate::client::ApiKey;
use serde_json::{Value, json};
use std::error::Error;

const ANTHROPIC: &str = include_str!("fixtures/anthropic.json");

fn fixture() -> Value {
    serde_json::from_str(ANTHROPIC).unwrap()
}

fn read(value: &Value) -> Completion {
    decode(
        Api::AnthropicMessages,
        &serde_json::to_vec(value).unwrap(),
        &Redactor::default(),
    )
}

#[test]
fn complete_message_retains_text_and_shared_accounting() {
    let answer = read(&fixture());
    assert_eq!(answer.stop, Some(ir::StopReason::EndTurn));
    assert_eq!(answer.text.unwrap().as_str(), r#"{"title":"Fix tests"}"#);
    assert_eq!(answer.usage.input, 17);
    assert_eq!(answer.usage.output, 7);
    assert_eq!(answer.usage.cache_read, 3);
    assert_eq!(answer.usage.cache_write, 2);
}

#[test]
fn missing_and_future_stops_do_not_inherit_the_streams_default() {
    for stop in [Value::Null, json!("future_reason")] {
        let mut body = fixture();
        body["stop_reason"] = stop;
        let answer = read(&body);
        assert_eq!(answer.stop, None);
        assert_eq!(answer.usage.output, 7);
        assert!(matches!(answer.text, Err(CompletionError::NotComplete)));
    }
    let mut body = fixture();
    assert!(
        body.as_object_mut()
            .unwrap()
            .remove("stop_reason")
            .is_some()
    );
    assert!(matches!(
        read(&body).text,
        Err(CompletionError::NotComplete)
    ));
}

#[test]
fn every_known_abnormal_stop_is_refused_without_losing_usage() {
    for (wire, expected) in [
        ("max_tokens", ir::StopReason::MaxTokens),
        ("stop_sequence", ir::StopReason::StopSequence),
        ("pause_turn", ir::StopReason::PauseTurn),
        ("model_context_window_exceeded", ir::StopReason::MaxTokens),
    ] {
        let mut body = fixture();
        body["stop_reason"] = json!(wire);
        let answer = read(&body);
        assert_eq!(answer.stop, Some(expected));
        assert_eq!(answer.usage.input, 17);
        assert!(matches!(answer.text, Err(CompletionError::NotComplete)));
    }
    for (wire, expected) in [
        ("tool_use", ir::StopReason::ToolUse),
        ("refusal", ir::StopReason::Refusal),
    ] {
        let mut body = fixture();
        body["stop_reason"] = json!(wire);
        let answer = read(&body);
        assert_eq!(answer.stop, Some(expected));
        assert!(matches!(
            answer.text,
            Err(CompletionError::Tool | CompletionError::Refusal)
        ));
    }
}

#[test]
fn tool_content_overrides_a_contradictory_normal_ending() {
    for block in [
        json!({"type":"tool_use","id":"call_fixture","name":"tool","input":{}}),
        json!({"type":"tool_result","tool_use_id":"call_fixture","content":"result"}),
    ] {
        let mut body = fixture();
        body["content"].as_array_mut().unwrap().push(block);
        let answer = read(&body);
        assert_eq!(answer.stop, Some(ir::StopReason::ToolUse));
        assert_eq!(answer.usage.output, 7);
        assert!(matches!(answer.text, Err(CompletionError::Tool)));
    }
}

#[test]
fn nonassistant_unknown_content_and_stop_sequences_are_not_plain_answers() {
    let mut role = fixture();
    role["role"] = json!("user");
    let mut kind = fixture();
    kind["content"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"future_output"}));
    let mut sequence = fixture();
    sequence["stop_sequence"] = json!("unexpected");
    for body in [role, kind, sequence] {
        assert!(matches!(
            read(&body).text,
            Err(CompletionError::Unsupported)
        ));
    }
}

#[test]
fn reasoning_only_and_blank_text_are_empty() {
    for content in [
        json!([]),
        json!([{"type":"redacted_thinking","data":"opaque"}]),
        json!([{"type":"text","text":" \n "}]),
    ] {
        let mut body = fixture();
        body["content"] = content;
        assert!(matches!(read(&body).text, Err(CompletionError::Empty)));
    }
}

#[test]
fn bound_is_checked_before_any_response_decoding() {
    let mut exact = ANTHROPIC.as_bytes().to_vec();
    exact.resize(MAX_COMPLETION_BYTES, b' ');
    assert!(
        decode(Api::AnthropicMessages, &exact, &Redactor::default())
            .text
            .is_ok()
    );
    exact.push(b' ');
    assert!(matches!(
        decode(Api::AnthropicMessages, &exact, &Redactor::default()).text,
        Err(CompletionError::TooLarge)
    ));
    assert!(
        CompletionError::TooLarge
            .to_string()
            .contains(&MAX_COMPLETION_BYTES.to_string())
    );
    assert!(matches!(
        decode(Api::AnthropicMessages, &[0xff], &Redactor::default()).text,
        Err(CompletionError::NotText(_))
    ));
}

#[test]
fn error_objects_and_unimplemented_apis_are_typed_refusals() {
    assert!(matches!(
        read(&json!({"error":{"type":"overloaded","message":"private"}})).text,
        Err(CompletionError::Failed)
    ));
    let mut complete_error = fixture();
    complete_error["error"] = json!({"message":"private"});
    let answer = read(&complete_error);
    assert!(matches!(answer.text, Err(CompletionError::Failed)));
    assert_eq!(answer.usage.output, 7);
    assert_eq!(answer.stop, Some(ir::StopReason::EndTurn));
    for api in [Api::OpenAiChat, Api::OpenAiResponses] {
        assert!(
            matches!(decode(api, ANTHROPIC.as_bytes(), &Redactor::default()).text, Err(CompletionError::UnsupportedApi(found)) if found == api)
        );
    }
}

#[test]
fn key_echoes_are_checked_before_parse_and_after_text_decoding() {
    let redactor = Redactor::new(Some(&ApiKey::new("fixture-credential".into())));
    for text in [
        "fixture-credential",
        r"fixture-\u0063redential",
        r#"{"nested":"fixture-\u0063redential"}"#,
    ] {
        let mut body = fixture();
        body["content"] = json!([{"type":"text","text":text}]);
        let answer = decode(
            Api::AnthropicMessages,
            &serde_json::to_vec(&body).unwrap(),
            &redactor,
        );
        assert!(matches!(answer.text, Err(CompletionError::QuotedKey)));
    }
    let mut body = fixture();
    body["fixture-credential"] = json!("ignored field");
    assert!(matches!(
        decode(
            Api::AnthropicMessages,
            &serde_json::to_vec(&body).unwrap(),
            &redactor
        )
        .text,
        Err(CompletionError::QuotedKey)
    ));
}

#[test]
fn diagnostics_never_reopen_the_serde_payload_or_success_text() {
    let mut body = fixture();
    body["role"] = json!("private-marker");
    let answer = read(&body);
    let error = answer.text.unwrap_err();
    assert!(!format!("{error:?} {error}").contains("private-marker"));
    let cause = error.source().unwrap();
    assert!(!format!("{cause:?} {cause}").contains("private-marker"));
    assert!(cause.source().is_none());
    let text = read(&fixture()).text.unwrap();
    assert_eq!(format!("{text:?}"), "CompletionText(..)");
}
