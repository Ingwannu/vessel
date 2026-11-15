//! Synthetic whole `OpenAI` replies and refusal boundaries, never a live provider.
use super::*;
use serde_json::{Value, json};

fn chat() -> Value {
    serde_json::from_str(include_str!("fixtures/chat.json")).unwrap()
}
fn responses() -> Value {
    serde_json::from_str(include_str!("fixtures/responses.json")).unwrap()
}
fn read(api: Api, value: &Value) -> Completion {
    decode(
        api,
        &serde_json::to_vec(value).unwrap(),
        &Redactor::default(),
    )
}
// These fixtures carry exactly the two whole-reply shapes tested here;
// the production Api enum needs no impossible test-only branch.
enum OpenReply {
    Chat(Value),
    Responses(Value),
}
impl OpenReply {
    fn cases() -> impl Iterator<Item = Self> {
        [Self::Chat(chat()), Self::Responses(responses())].into_iter()
    }
    fn message(&mut self) -> &mut Value {
        match self {
            Self::Chat(body) => &mut body["choices"][0]["message"],
            Self::Responses(body) => body["output"].as_array_mut().unwrap().last_mut().unwrap(),
        }
    }
    fn read(&self, redactor: &Redactor) -> Completion {
        let (api, value) = match self {
            Self::Chat(body) => (Api::OpenAiChat, body),
            Self::Responses(body) => (Api::OpenAiResponses, body),
        };
        decode(api, &serde_json::to_vec(value).unwrap(), redactor)
    }
}

#[test]
fn whole_openai_accounting_uses_the_stream_convention() {
    for (api, body) in [
        (Api::OpenAiChat, chat()),
        (Api::OpenAiResponses, responses()),
    ] {
        let answer = read(api, &body);
        assert_eq!(answer.stop, Some(ir::StopReason::EndTurn));
        assert_eq!(answer.text.unwrap().as_str(), "answer");
        assert_eq!(answer.usage.input, 14);
        assert_eq!(answer.usage.cache_read, 3);
        assert_eq!(answer.usage.output, 7);
        assert_eq!(answer.usage.reasoning, 2);
    }
}
#[test]
fn no_choice_or_multiple_choices_are_never_selected() {
    for count in [0, COMPLETION_OUTPUTS + 1] {
        let mut body = chat();
        let choice = body["choices"][0].clone();
        body["choices"] = Value::Array(std::iter::repeat_n(choice, count).collect());
        let answer = read(Api::OpenAiChat, &body);
        assert!(matches!(answer.text, Err(CompletionError::Outputs)));
        assert_eq!(answer.usage.output, 7);
    }
}
#[test]
fn chat_requires_an_explicit_normal_ending() {
    for finish in [
        Value::Null,
        json!("future_stop"),
        json!("length"),
        json!("error"),
    ] {
        let mut body = chat();
        body["choices"][0]["finish_reason"] = finish;
        assert!(read(Api::OpenAiChat, &body).text.is_err());
    }
}
#[test]
fn chat_cannot_hide_tools_or_refusals_behind_stop() {
    for (field, value) in [
        ("function_call", json!({"name":"tool","arguments":"{}"})),
        (
            "tool_calls",
            json!([{"type":"function","id":"call_fixture","function":{"name":"tool","arguments":"{}"}}]),
        ),
        ("refusal", json!("private refusal")),
    ] {
        let mut body = chat();
        body["choices"][0]["message"][field] = value;
        let answer = read(Api::OpenAiChat, &body);
        assert!(matches!(
            answer.text,
            Err(CompletionError::Tool | CompletionError::Refusal)
        ));
        assert_eq!(answer.usage.output, 7);
    }
}
#[test]
fn response_unknown_or_incomplete_status_cannot_be_repaired_by_text() {
    for status in [
        "in_progress",
        "incomplete",
        "future_status",
        "failed",
        "cancelled",
        "queued",
    ] {
        let mut body = responses();
        body["status"] = json!(status);
        assert!(read(Api::OpenAiResponses, &body).text.is_err());
    }
    for reason in ["max_output_tokens", "content_filter", "future_reason"] {
        let mut body = responses();
        body["incomplete_details"] = json!({"reason":reason});
        let answer = read(Api::OpenAiResponses, &body);
        assert!(answer.text.is_err());
        assert_eq!(answer.usage.output, 7);
    }
}
#[test]
fn response_refuses_mixed_tools_unknown_items_and_refusals() {
    for item in [
        json!({"type":"future_output"}),
        json!({"type":"function_call","call_id":"call_fixture","name":"tool","arguments":"{}"}),
    ] {
        let mut body = responses();
        body["output"].as_array_mut().unwrap().push(item);
        assert!(read(Api::OpenAiResponses, &body).text.is_err());
    }
    let mut body = responses();
    body["output"].as_array_mut().unwrap().last_mut().unwrap()["content"] =
        json!([{"type":"refusal","refusal":"private"}]);
    assert!(matches!(
        read(Api::OpenAiResponses, &body).text,
        Err(CompletionError::Refusal)
    ));
}

#[test]
fn response_message_count_is_not_inferred_from_reasoning_items() {
    for count in [0, COMPLETION_OUTPUTS + 1] {
        let mut body = responses();
        let message = body["output"].as_array_mut().unwrap().pop().unwrap();
        let outputs = body["output"].as_array_mut().unwrap();
        outputs.extend(std::iter::repeat_n(message, count));
        let answer = read(Api::OpenAiResponses, &body);
        assert!(matches!(answer.text, Err(CompletionError::Outputs)));
        assert_eq!(answer.usage.output, 7);
    }
}
#[test]
fn incomplete_or_nonassistant_messages_and_unknown_parts_are_refused() {
    for status in ["in_progress", "incomplete", "future_status"] {
        let mut body = responses();
        body["output"].as_array_mut().unwrap().last_mut().unwrap()["status"] = json!(status);
        assert!(matches!(
            read(Api::OpenAiResponses, &body).text,
            Err(CompletionError::Unsupported)
        ));
    }
    for mut body in OpenReply::cases() {
        body.message()["content"] = json!([{"type":"future_content"}]);
        assert!(matches!(
            body.read(&Redactor::default()).text,
            Err(CompletionError::Unsupported)
        ));
    }
    let mut body = chat();
    body["choices"][0]["message"]["role"] = json!("user");
    assert!(matches!(
        read(Api::OpenAiChat, &body).text,
        Err(CompletionError::Decode(_))
    ));
    let mut body = responses();
    body["output"].as_array_mut().unwrap().last_mut().unwrap()["role"] = json!("user");
    assert!(matches!(
        read(Api::OpenAiResponses, &body).text,
        Err(CompletionError::Unsupported)
    ));
}
#[test]
fn missing_usage_is_unknown_and_declared_error_keeps_reported_counts() {
    for (api, mut body) in [
        (Api::OpenAiChat, chat()),
        (Api::OpenAiResponses, responses()),
    ] {
        body["error"] = json!({"message":"private"});
        let answer = read(api, &body);
        assert!(matches!(answer.text, Err(CompletionError::Failed)));
        assert_eq!(answer.usage.output, 7);
        assert!(body.as_object_mut().unwrap().remove("error").is_some());
        assert!(body.as_object_mut().unwrap().remove("usage").is_some());
        let answer = read(api, &body);
        assert!(answer.text.is_ok());
        assert_eq!(answer.usage, ir::Usage::default());
    }
}
#[test]
fn whole_openai_text_never_bypasses_credential_checks() {
    let key = crate::client::ApiKey::new("fixture-credential".into());
    let redactor = Redactor::new(Some(&key));
    for mut body in OpenReply::cases() {
        let nested = r#"{"title":"fixture-\u0063redential"}"#;
        body.message()["content"] = json!(nested);
        let answer = body.read(&redactor);
        assert!(matches!(answer.text, Err(CompletionError::QuotedKey)));
        assert!(!format!("{answer:?}").contains("fixture-credential"));
    }
}

#[test]
fn plain_parts_and_plain_response_text_use_the_same_finality_guard() {
    let mut body = chat();
    body["choices"][0]["message"]["content"] = json!([
        {"type":"text","text":"ans"},{"type":"text","text":"wer"}]);
    assert_eq!(
        read(Api::OpenAiChat, &body).text.unwrap().as_str(),
        "answer"
    );
    let mut body = responses();
    body["output"].as_array_mut().unwrap().last_mut().unwrap()["content"] = json!("answer");
    assert_eq!(
        read(Api::OpenAiResponses, &body).text.unwrap().as_str(),
        "answer"
    );
}
#[test]
fn known_openai_stops_keep_the_reported_meaning() {
    for (wire, expected) in [
        ("length", ir::StopReason::MaxTokens),
        ("tool_calls", ir::StopReason::ToolUse),
        ("content_filter", ir::StopReason::Refusal),
    ] {
        let mut body = chat();
        body["choices"][0]["finish_reason"] = json!(wire);
        let answer = read(Api::OpenAiChat, &body);
        assert_eq!(answer.stop, Some(expected));
        assert!(answer.text.is_err());
    }
    for (wire, expected) in [
        ("max_output_tokens", ir::StopReason::MaxTokens),
        ("content_filter", ir::StopReason::Refusal),
    ] {
        let mut body = responses();
        body["status"] = json!("incomplete");
        body["incomplete_details"] = json!({"reason":wire});
        let answer = read(Api::OpenAiResponses, &body);
        assert_eq!(answer.stop, Some(expected));
        assert!(answer.text.is_err());
    }
}
