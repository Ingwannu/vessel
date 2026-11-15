//! The timeline a turn's frames give, as the CLI sends them with
//! `--include-partial-messages`.

use serde_json::json;
use vessel_proto::{
    AgentEvent, ContentBlock, StopReason, ToolCallContent, ToolCallStatus, ToolKind,
};

use super::Translator;
use crate::claude::wire::{Frame, TurnResult};

fn read(value: &serde_json::Value) -> Frame {
    serde_json::from_value(value.clone()).unwrap()
}

/// What `frames` show, in order.
fn shown(translator: &mut Translator, frames: &[serde_json::Value]) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    for frame in frames {
        match read(frame) {
            Frame::StreamEvent(frame) => events.extend(translator.stream(&frame)),
            Frame::Assistant(frame) => events.extend(translator.assistant(&frame)),
            Frame::User(frame) => events.extend(translator.user(&frame)),
            Frame::System(_)
            | Frame::Result(_)
            | Frame::ControlRequest(_)
            | Frame::ControlResponse(_)
            | Frame::ControlCancelRequest(_)
            | Frame::KeepAlive
            | Frame::Unknown => panic!("not a message frame: {frame}"),
        }
    }
    events
}

fn text(text: &str) -> ContentBlock {
    ContentBlock::Text {
        text: text.to_owned(),
    }
}

fn stream(event: &serde_json::Value) -> serde_json::Value {
    json!({"type": "stream_event", "uuid": "u", "session_id": "s", "parent_tool_use_id": null,
           "event": event})
}

#[test]
fn streamed_text_shows_once_and_a_tool_call_is_announced_then_settled() {
    let mut translator = Translator::default();
    let events = shown(
        &mut translator,
        &[
            stream(&json!({"type": "message_start", "message": {"id": "msg_1", "content": []}})),
            stream(&json!({"type": "content_block_start", "index": 0,
                           "content_block": {"type": "thinking", "thinking": ""}})),
            stream(&json!({"type": "content_block_delta", "index": 0,
                           "delta": {"type": "thinking_delta", "thinking": "Look first."}})),
            stream(&json!({"type": "content_block_delta", "index": 1,
                           "delta": {"type": "text_delta", "text": "Listing "}})),
            stream(&json!({"type": "content_block_delta", "index": 1,
                           "delta": {"type": "text_delta", "text": "files."}})),
            // The whole message repeats what streamed, and adds the call.
            json!({"type": "assistant", "uuid": "a1", "session_id": "s", "parent_tool_use_id": null,
                   "message": {"id": "msg_1", "content": [
                       {"type": "thinking", "thinking": "Look first.", "signature": "x"},
                       {"type": "text", "text": "Listing files."},
                       {"type": "tool_use", "id": "toolu_1", "name": "Bash",
                        "input": {"command": "ls -la\necho done", "description": "List"}}]}}),
            // A repeated frame for the same call announces nothing new.
            json!({"type": "assistant", "parent_tool_use_id": null,
                   "message": {"id": "msg_1", "content": [
                       {"type": "tool_use", "id": "toolu_1", "name": "Bash", "input": {}}]}}),
            json!({"type": "user", "parent_tool_use_id": null, "isSynthetic": true,
                   "message": {"role": "user", "content": [
                       {"type": "tool_result", "tool_use_id": "toolu_1",
                        "content": [{"type": "text", "text": "a\nb"}], "is_error": false}]}}),
        ],
    );
    assert_eq!(events.len(), 5, "{events:#?}");
    assert_eq!(
        events[0],
        AgentEvent::ThoughtChunk {
            content: text("Look first.")
        }
    );
    assert_eq!(
        events[1..3],
        [
            AgentEvent::MessageChunk {
                content: text("Listing ")
            },
            AgentEvent::MessageChunk {
                content: text("files.")
            }
        ]
    );
    let AgentEvent::ToolCall(call) = &events[3] else {
        panic!("{:?}", events[3]);
    };
    assert_eq!(call.id.as_str(), "toolu_1");
    assert_eq!(call.kind, ToolKind::Execute);
    assert_eq!(call.title, "Bash: ls -la");
    assert_eq!(call.status, ToolCallStatus::Pending);
    let AgentEvent::ToolCallUpdate(update) = &events[4] else {
        panic!("{:?}", events[4]);
    };
    assert_eq!(update.status, Some(ToolCallStatus::Completed));
    assert_eq!(
        update.content,
        Some(vec![ToolCallContent::Content {
            content: text("a\nb")
        }])
    );
}

#[test]
fn a_message_that_never_streamed_shows_from_its_frame_and_a_subagent_stays_out() {
    let mut translator = Translator::default();
    let events = shown(
        &mut translator,
        &[
            json!({"type": "assistant", "parent_tool_use_id": null,
                   "message": {"id": "msg_local", "content": [{"type": "text", "text": "Context: 12%"}]}}),
            json!({"type": "assistant", "parent_tool_use_id": "toolu_task",
                   "message": {"id": "msg_sub", "content": [
                       {"type": "text", "text": "from the subagent"},
                       {"type": "tool_use", "id": "toolu_9", "name": "Read",
                        "input": {"file_path": "/work/a.rs"}}]}}),
            stream(&json!({"type": "ping"})),
        ],
    );
    assert_eq!(
        events,
        [AgentEvent::MessageChunk {
            content: text("Context: 12%")
        }]
    );
}

#[test]
fn a_file_tool_names_its_file_and_a_failed_result_fails_the_call() {
    let mut translator = Translator::default();
    let events = shown(
        &mut translator,
        &[
            json!({"type": "assistant", "parent_tool_use_id": null,
                   "message": {"id": "msg_2", "content": [
                       {"type": "tool_use", "id": "toolu_2", "name": "Edit",
                        "input": {"file_path": "/work/a.rs", "old_string": "a", "new_string": "b"}}]}}),
            json!({"type": "user", "parent_tool_use_id": null,
                   "message": {"role": "user", "content": [
                       {"type": "tool_result", "tool_use_id": "toolu_2", "content": "no such file",
                        "is_error": true},
                       {"type": "tool_result", "tool_use_id": "toolu_unknown", "content": "x"}]}}),
        ],
    );
    let AgentEvent::ToolCall(call) = &events[0] else {
        panic!("{events:?}");
    };
    assert_eq!(call.kind, ToolKind::Edit);
    assert_eq!(
        call.locations[0].path,
        std::path::PathBuf::from("/work/a.rs")
    );
    let AgentEvent::ToolCallUpdate(update) = &events[1] else {
        panic!("{events:?}");
    };
    assert_eq!(update.status, Some(ToolCallStatus::Failed));
    assert_eq!(
        events.len(),
        2,
        "a result for no announced call shows nothing"
    );
}

fn result(value: &serde_json::Value) -> TurnResult {
    let Frame::Result(result) = read(value) else {
        panic!("not a result");
    };
    result
}

#[test]
fn the_result_ends_the_turn_or_fails_it_in_the_agents_words() {
    let done = result(
        &json!({"type": "result", "subtype": "success", "is_error": false,
        "result": "ok", "stop_reason": "end_turn",
        "usage": {"input_tokens": 3, "output_tokens": 5, "cache_read_input_tokens": 100,
                  "cache_creation_input_tokens": 2}}),
    );
    let AgentEvent::TurnEnded { reason, usage, .. } = Translator::end(&done, false) else {
        panic!("not ended");
    };
    assert_eq!(reason, StopReason::EndTurn);
    let usage = usage.unwrap();
    assert_eq!(
        (usage.input.get(), usage.output.get(), usage.total.get()),
        (105, 5, 110)
    );

    let limited = result(
        &json!({"type": "result", "subtype": "success", "is_error": true,
        "result": "  You've hit your usage limit  "}),
    );
    assert_eq!(
        Translator::end(&limited, false),
        AgentEvent::TurnFailed {
            message: "You've hit your usage limit".to_owned()
        }
    );
    let crashed = result(
        &json!({"type": "result", "subtype": "error_during_execution",
        "is_error": true, "errors": ["boom", "again"]}),
    );
    assert_eq!(
        Translator::end(&crashed, false),
        AgentEvent::TurnFailed {
            message: "boom; again".to_owned()
        }
    );
    let stopped = result(
        &json!({"type": "result", "subtype": "success", "is_error": false,
        "stop_reason": null}),
    );
    assert!(matches!(
        Translator::end(&stopped, true),
        AgentEvent::TurnEnded {
            reason: StopReason::Cancelled,
            ..
        }
    ));
}
