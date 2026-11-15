//! Frames as the Agent SDK's declared types shape them (written here from
//! the field names, not copied), and what a newer CLI may add.

use serde_json::json;

use super::*;

fn frame(value: &Value) -> Frame {
    serde_json::from_value(value.clone()).unwrap()
}

#[test]
fn init_names_the_conversation_and_what_the_cli_can_do() {
    let init = frame(&json!({
        "type": "system",
        "subtype": "init",
        "session_id": "5d0c",
        "uuid": "u1",
        "apiKeySource": "none",
        "claude_code_version": "2.1.246",
        "cwd": "/work",
        "tools": ["Read", "Bash"],
        "mcp_servers": [],
        "model": "claude-opus-5-5",
        "permissionMode": "default",
        "slash_commands": ["compact", "review"],
        "output_style": "default",
        "skills": [],
        "plugins": [],
        "capabilities": ["interrupt_receipt_v1"],
        "a_field_from_later": true
    }));
    let Frame::System(System::Init(init)) = init else {
        panic!("{init:?}");
    };
    assert_eq!(init.session_id, "5d0c");
    assert_eq!(init.permission_mode.as_deref(), Some("default"));
    assert_eq!(init.slash_commands, ["compact", "review"]);
    assert_eq!(init.capabilities, ["interrupt_receipt_v1"]);
}

#[test]
fn a_streamed_reply_reads_block_by_block() {
    let start = frame(&json!({
        "type": "stream_event",
        "uuid": "u2",
        "session_id": "5d0c",
        "parent_tool_use_id": null,
        "event": {"type": "content_block_start", "index": 1,
                  "content_block": {"type": "tool_use", "id": "toolu_1", "name": "Bash", "input": {}}}
    }));
    assert_eq!(
        start,
        Frame::StreamEvent(StreamEvent {
            event: RawEvent::ContentBlockStart {
                index: 1,
                content_block: Block::ToolUse {
                    id: "toolu_1".to_owned(),
                    name: "Bash".to_owned(),
                    input: json!({}),
                },
            },
            parent_tool_use_id: None,
        })
    );
    let delta = frame(&json!({
        "type": "stream_event", "uuid": "u3", "session_id": "5d0c", "parent_tool_use_id": null,
        "event": {"type": "content_block_delta", "index": 0,
                  "delta": {"type": "text_delta", "text": "Hel"}}
    }));
    let Frame::StreamEvent(StreamEvent {
        event:
            RawEvent::ContentBlockDelta {
                index: 0,
                delta: Delta::Text { text },
            },
        ..
    }) = delta
    else {
        panic!("{delta:?}");
    };
    assert_eq!(text, "Hel");
    for unknown in [
        json!({"type": "stream_event", "event": {"type": "ping"}}),
        json!({"type": "stream_event", "event": {"type": "message_delta", "delta": {"stop_reason": "end_turn"}}}),
        json!({"type": "stream_event", "event": {"type": "content_block_delta", "index": 0,
               "delta": {"type": "signature_delta", "signature": "abc"}}}),
    ] {
        let read = frame(&unknown);
        assert!(
            matches!(
                read,
                Frame::StreamEvent(StreamEvent {
                    event: RawEvent::Unknown
                        | RawEvent::ContentBlockDelta {
                            delta: Delta::Unknown,
                            ..
                        },
                    ..
                })
            ),
            "{read:?}"
        );
    }
}

#[test]
fn a_tool_result_comes_in_a_user_frame_as_text_or_blocks() {
    let text = frame(&json!({
        "type": "user", "session_id": "5d0c", "parent_tool_use_id": null, "isSynthetic": true,
        "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_1", "content": "done", "is_error": false}
        ]},
        "tool_use_result": {"stdout": "done"}
    }));
    let Frame::User(user) = text else {
        panic!("{text:?}");
    };
    assert!(user.synthetic);
    let ToolResultContent::Blocks(blocks) = user.message.content else {
        panic!("blocks");
    };
    assert_eq!(
        blocks,
        [Block::ToolResult {
            tool_use_id: "toolu_1".to_owned(),
            content: ToolResultContent::Text("done".to_owned()),
            is_error: false,
        }]
    );
    let replayed = frame(&json!({
        "type": "user", "parent_tool_use_id": null,
        "message": {"role": "user", "content": "earlier question"}
    }));
    let Frame::User(replayed) = replayed else {
        panic!("{replayed:?}");
    };
    assert_eq!(
        replayed.message.content,
        ToolResultContent::Text("earlier question".to_owned())
    );
}

#[test]
fn a_result_ends_the_turn_with_its_usage_or_its_errors() {
    let done = frame(&json!({
        "type": "result", "subtype": "success", "is_error": false, "result": "Hello",
        "duration_ms": 10, "duration_api_ms": 8, "num_turns": 1, "stop_reason": "end_turn",
        "total_cost_usd": 0.01, "modelUsage": {}, "permission_denials": [],
        "usage": {"input_tokens": 3, "output_tokens": 5, "cache_read_input_tokens": 7,
                  "cache_creation_input_tokens": 0, "server_tool_use": {}},
        "uuid": "u9", "session_id": "5d0c"
    }));
    let Frame::Result(done) = done else {
        panic!("{done:?}");
    };
    assert!(!done.is_error);
    assert_eq!(done.stop_reason.as_deref(), Some("end_turn"));
    assert_eq!(
        done.usage.map(|usage| usage.cache_read_input_tokens),
        Some(7)
    );
    let failed = frame(&json!({
        "type": "result", "subtype": "error_during_execution", "is_error": true,
        "errors": ["the API refused"], "uuid": "u10", "session_id": "5d0c"
    }));
    let Frame::Result(failed) = failed else {
        panic!("{failed:?}");
    };
    assert_eq!(failed.errors, ["the API refused"]);
}

#[test]
fn control_frames_read_and_unknown_requests_stay_unknown() {
    let asked = frame(&json!({
        "type": "control_request", "request_id": "r1",
        "request": {"subtype": "can_use_tool", "tool_name": "Bash",
                    "input": {"command": "ls"}, "tool_use_id": "toolu_1",
                    "permission_suggestions": [{"type": "addRules", "rules": [], "behavior": "allow",
                                                "destination": "session"}]}
    }));
    let Frame::ControlRequest(ControlRequest {
        request_id,
        request: AgentRequest::CanUseTool(can),
    }) = asked
    else {
        panic!("{asked:?}");
    };
    assert_eq!(request_id, "r1");
    assert_eq!(can.tool_name, "Bash");
    assert_eq!(can.permission_suggestions.len(), 1);
    let hook = frame(&json!({
        "type": "control_request", "request_id": "r2",
        "request": {"subtype": "hook_callback", "callback_id": "h"}
    }));
    assert!(matches!(
        hook,
        Frame::ControlRequest(ControlRequest {
            request: AgentRequest::Unknown,
            ..
        })
    ));
    assert_eq!(
        frame(&json!({"type": "control_cancel_request", "request_id": "r1"})),
        Frame::ControlCancelRequest(ControlCancel {
            request_id: "r1".to_owned()
        })
    );
    assert_eq!(frame(&json!({"type": "keep_alive"})), Frame::KeepAlive);
    assert_eq!(
        frame(&json!({"type": "rate_limit_event", "anything": 1})),
        Frame::Unknown
    );
    let answered = frame(&json!({
        "type": "control_response",
        "response": {"subtype": "error", "request_id": "h1", "error": "unknown subtype"}
    }));
    assert_eq!(
        answered,
        Frame::ControlResponse(ControlResponse {
            response: Answer::Error {
                request_id: "h1".to_owned(),
                error: "unknown subtype".to_owned()
            }
        })
    );
}

#[test]
fn what_the_host_writes_is_shaped_as_the_cli_reads_it() {
    let prompt = Outgoing::User {
        message: OutgoingMessage {
            role: "user",
            content: vec![OutgoingBlock::Text {
                text: "hi".to_owned(),
            }],
        },
        parent_tool_use_id: None,
        session_id: "5d0c".to_owned(),
    };
    assert_eq!(
        serde_json::to_value(&prompt).unwrap(),
        json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": "hi"}]},
               "parent_tool_use_id": null, "session_id": "5d0c"})
    );
    let initialize = Outgoing::ControlRequest {
        request_id: "h0".to_owned(),
        request: HostRequest::Initialize {},
    };
    assert_eq!(
        serde_json::to_value(&initialize).unwrap(),
        json!({"type": "control_request", "request_id": "h0", "request": {"subtype": "initialize"}})
    );
    let interrupt = Outgoing::ControlRequest {
        request_id: "h1".to_owned(),
        request: HostRequest::Interrupt {},
    };
    assert_eq!(
        serde_json::to_value(&interrupt).unwrap(),
        json!({"type": "control_request", "request_id": "h1", "request": {"subtype": "interrupt"}})
    );
    let deny = Outgoing::ControlResponse {
        response: HostAnswer::Success {
            request_id: "r1".to_owned(),
            response: serde_json::to_value(Permission::Deny {
                message: "no".to_owned(),
                tool_use_id: "toolu_1".to_owned(),
            })
            .unwrap(),
        },
    };
    assert_eq!(
        serde_json::to_value(&deny).unwrap(),
        json!({"type": "control_response", "response": {"subtype": "success", "request_id": "r1",
               "response": {"behavior": "deny", "message": "no", "toolUseID": "toolu_1"}}})
    );
}
