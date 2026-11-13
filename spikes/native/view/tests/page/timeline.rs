//! The page's `data/timeline.test.ts`, `timeline-agent.test.ts` and
//! `timeline.recorded.test.ts`, case for case.

use serde_json::json;
use vessel_app_view::timeline::{EntryKind, Timeline};
use vessel_proto::{EventRecord, EventSeq, PermissionOutcome, ToolCallStatus};

use crate::support::{Log, folded, key, kinds, uuid};

const QWEN: &str = r#"{
    "route": "qwen-coder",
    "model": "qwen/qwen3-coder",
    "api": "openai_chat_completions",
    "usage": { "input": 72, "cache_read": 128, "cache_write": 0, "output": 50, "reasoning": 10 },
    "stop": "tool_use",
    "duration_ms": 1234
}"#;

/// One `fake-acp` turn, as its doc describes it, up to the permission.
fn turn(log: &mut Log) -> Vec<EventRecord> {
    log.seq = 0;
    vec![
        log.record(&json!({ "type": "status", "status": { "state": "running" } })),
        log.agent(&json!({ "event": "ready", "harness_session": "fake-session-1" })),
        log.agent(&json!({ "event": "turn_started", "content": [{ "type": "text", "text": "fix it" }] })),
        log.agent(&json!({ "event": "thought_chunk", "content": { "type": "text", "text": "thinking" } })),
        log.agent(&json!({ "event": "message_chunk", "content": { "type": "text", "text": "echo: fix it " } })),
        log.agent(&json!({ "event": "message_chunk", "content": { "type": "text", "text": "(model fake-small)" } })),
        log.agent(&json!({
            "event": "tool_call", "id": "call-1", "title": "Read status.rs", "kind": "read",
            "status": "pending", "content": [],
            "locations": [{ "path": "/src/app/crates/fleet/src/status.rs", "line": 12 }],
            "raw_input": { "path": "status.rs" }
        })),
        log.agent(&json!({
            "event": "plan",
            "entries": [{ "content": "Read the code", "priority": "high", "status": "in_progress" }]
        })),
        log.agent(&json!({ "event": "tool_call_update", "id": "call-1", "status": "in_progress" })),
        log.agent(&json!({
            "event": "permission_requested", "id": uuid(1), "tool_call": "call-1",
            "options": [
                { "id": "allow-once", "name": "Allow once", "kind": "allow_once" },
                { "id": "reject-once", "name": "Reject once", "kind": "reject_once" }
            ]
        })),
    ]
}

fn pending_title(timeline: &Timeline) -> Option<String> {
    timeline
        .pending()
        .and_then(|(_, permission)| permission.title.clone())
}

#[test]
fn folds_a_turn_into_entries_merging_chunks_and_updating_tools_in_place() {
    let timeline = folded(&turn(&mut Log::default()));
    assert_eq!(
        kinds(&timeline),
        [
            "status",
            "ready",
            "prompt",
            "thought",
            "message",
            "tool",
            "plan",
            "permission"
        ]
    );
    let entries = timeline.entries();
    assert!(
        matches!(&entries[4].kind, EntryKind::Message { text } if text == "echo: fix it (model fake-small)")
    );
    assert!(
        matches!(&entries[5].kind, EntryKind::Tool(tool) if tool.status == ToolCallStatus::InProgress)
    );
    assert_eq!(key(&entries[5]), "tcall-1");
    assert_eq!(timeline.turns(), 1);
    assert!(timeline.in_turn());
    assert_eq!(pending_title(&timeline).as_deref(), Some("Read status.rs"));
    assert_eq!(
        timeline.files(),
        [std::path::PathBuf::from(
            "/src/app/crates/fleet/src/status.rs"
        )]
    );
    assert_eq!(timeline.plan().unwrap()[0].content, "Read the code");
    assert_eq!(timeline.next(), EventSeq::new(10));
}

#[test]
fn resolves_a_permission_and_ends_the_turn() {
    let mut log = Log::default();
    let mut timeline = folded(&turn(&mut log));
    let prompt = timeline.entries()[2].clone();
    let change = timeline.apply(&[
        log.agent(&json!({
            "event": "permission_resolved", "id": uuid(1),
            "outcome": { "outcome": "selected", "option": "allow-once" }
        })),
        log.agent(&json!({
            "event": "tool_call_update", "id": "call-1", "status": "completed",
            "raw_output": { "ok": true }
        })),
        log.agent(&json!({ "event": "turn_ended", "reason": "end_turn" })),
    ]);
    assert!(timeline.pending().is_none());
    assert!(!timeline.in_turn());
    let permission = timeline
        .entries()
        .iter()
        .find_map(|entry| entry.kind.as_permission())
        .unwrap();
    assert!(matches!(
        &permission.outcome,
        Some(PermissionOutcome::Selected { option }) if option.as_str() == "allow-once"
    ));
    // Entries that did not change are as they were, and the batch names
    // only what it changed (the page: the same objects).
    assert_eq!(timeline.entries()[2], prompt);
    assert_eq!(change.changed, [5, 7]);
    assert_eq!(change.added_from, 8);
    assert_eq!(
        crate::support::kind(timeline.entries().last().unwrap()),
        "turn_ended"
    );
}

#[test]
fn finds_tool_calls_and_permissions_by_id_across_batches() {
    let mut log = Log::default();
    let mut records: Vec<EventRecord> = (0..300)
        .map(|n| {
            log.agent(&json!({
                "event": "tool_call", "id": format!("call-{n}"), "title": format!("Step {n}"),
                "kind": "read", "status": "pending", "content": [],
                "locations": [{ "path": format!("/src/file-{}.rs", n % 7), "line": n }]
            }))
        })
        .collect();
    for (permission, call) in [(10, "call-10"), (11, "call-250")] {
        records.push(log.agent(&json!({
            "event": "permission_requested", "id": uuid(permission), "tool_call": call,
            "options": [{ "id": "allow-once", "name": "Allow", "kind": "allow_once" }]
        })));
    }
    let mut timeline = folded(&records);
    let pending_id = |timeline: &Timeline| {
        timeline
            .pending()
            .map(|(_, permission)| permission.request.id.to_string())
    };
    assert_eq!(pending_id(&timeline), Some(uuid(10)));
    assert_eq!(pending_title(&timeline).as_deref(), Some("Step 10"));
    assert_eq!(timeline.files().len(), 7);
    let _change = timeline.apply(&[
        log.agent(&json!({ "event": "tool_call_update", "id": "call-250", "status": "completed" })),
        log.agent(&json!({
            "event": "permission_resolved", "id": uuid(10),
            "outcome": { "outcome": "selected", "option": "allow-once" }
        })),
    ]);
    let tools: Vec<_> = timeline
        .entries()
        .iter()
        .filter_map(|entry| entry.kind.as_tool())
        .collect();
    assert_eq!(tools.len(), 300);
    let updated = tools
        .iter()
        .find(|tool| tool.id.as_str() == "call-250")
        .unwrap();
    assert_eq!(updated.status, ToolCallStatus::Completed);
    assert_eq!(updated.title, "Step 250");
    assert_eq!(pending_id(&timeline), Some(uuid(11)));
}

#[test]
fn skips_records_it_has_seen_so_a_resumed_stream_can_overlap() {
    let records = turn(&mut Log::default());
    let mut once = folded(&records);
    let entries = once.entries().to_vec();
    let again = once.apply(&records);
    assert!(again.is_empty(once.entries().len()));
    assert_eq!(once.entries(), entries.as_slice());
    let mut halves = folded(&records[..5]);
    let _rest = halves.apply(&records);
    assert_eq!(halves.entries(), entries.as_slice());
}

#[test]
fn marks_user_chunks_outside_a_turn_as_a_replayed_prompt() {
    let mut log = Log::default();
    let timeline = folded(&[
        log.agent(&json!({ "event": "user_message_chunk", "content": { "type": "text", "text": "earlier " } })),
        log.agent(&json!({ "event": "user_message_chunk", "content": { "type": "text", "text": "question" } })),
        log.agent(&json!({ "event": "message_chunk", "content": { "type": "text", "text": "replayed" } })),
    ]);
    let EntryKind::Prompt(prompt) = &timeline.entries()[0].kind else {
        panic!("a prompt first");
    };
    assert!(prompt.replayed);
    assert_eq!(prompt.text, "earlier question");
    assert_eq!(timeline.entries()[0].rev, 1, "grown once, in place");
}

#[test]
fn lists_model_calls_and_sums_them_failures_counted_apart() {
    let mut log = Log::default();
    let qwen: serde_json::Value = serde_json::from_str(QWEN).unwrap();
    let mut flash = qwen.clone();
    flash["model"] = json!("qwen/qwen3-coder-flash");
    flash["usage"] =
        json!({ "input": 8, "cache_read": 0, "cache_write": 4, "output": 2, "reasoning": 0 });
    flash["stop"] = json!("end_turn");
    flash["duration_ms"] = json!(300);
    let mut failed = qwen.clone();
    failed["usage"] =
        json!({ "input": 0, "cache_read": 0, "cache_write": 0, "output": 0, "reasoning": 0 });
    failed["stop"] = json!("error");
    failed["failure"] = json!("rate_limited");
    failed["duration_ms"] = json!(40);
    let timeline = folded(&[log.model(qwen), log.model(flash), log.model(failed)]);
    assert_eq!(kinds(&timeline), ["model", "model", "model"]);
    let EntryKind::Model(first) = &timeline.entries()[0].kind else {
        panic!("a model call first");
    };
    assert_eq!(first.route.to_string(), "qwen-coder");
    let usage = timeline.usage().unwrap();
    assert_eq!(
        (
            usage.calls,
            usage.failed,
            usage.input,
            usage.cache_read,
            usage.cache_write,
            usage.output,
            usage.reasoning,
            usage.ms
        ),
        (3, 1, 80, 128, 4, 52, 10, 1574)
    );
    let models: Vec<String> = usage.models.iter().map(ToString::to_string).collect();
    assert_eq!(models, ["qwen/qwen3-coder", "qwen/qwen3-coder-flash"]);
    assert!(folded(&turn(&mut Log::default())).usage().is_none());
}

#[test]
fn keeps_what_the_session_runs_with_and_its_title_as_state_not_entries() {
    let mut log = Log::default();
    let model = json!({
        "id": "model", "name": "Model", "category": "model", "kind": "select", "current": "opus",
        "values": [{ "id": "opus", "name": "Opus" }, { "id": "haiku", "name": "Haiku" }]
    });
    let fast = json!({ "id": "fast", "name": "Fast mode", "category": "other", "kind": "toggle", "on": false });
    let mut timeline = folded(&[
        log.record(&json!({ "type": "options_changed", "options": [model.clone(), fast] })),
        log.record(&json!({ "type": "renamed", "title": "Fix the flaky test" })),
    ]);
    assert!(timeline.entries().is_empty());
    assert_eq!(timeline.options().unwrap().len(), 2);
    assert_eq!(
        timeline.title().map(ToString::to_string).as_deref(),
        Some("Fix the flaky test")
    );
    // Each report carries the whole set: the last one replaces the rest.
    let mut haiku = model;
    haiku["current"] = json!("haiku");
    let _change = timeline
        .apply(&[log.record(&json!({ "type": "options_changed", "options": [haiku.clone()] }))]);
    let options: Vec<serde_json::Value> = timeline
        .options()
        .unwrap()
        .iter()
        .map(|option| serde_json::to_value(option).unwrap())
        .collect();
    assert_eq!(options, [haiku]);
    assert_eq!(
        timeline.title().map(ToString::to_string).as_deref(),
        Some("Fix the flaky test")
    );
}

#[test]
fn a_question_is_an_entry_that_becomes_its_outcome_in_place() {
    let mut log = Log::default();
    let mut timeline = folded(&[
        log.agent(
            &json!({ "event": "turn_started", "content": [{ "type": "text", "text": "go" }] }),
        ),
        log.agent(
            &json!({ "event": "question_asked", "id": uuid(1), "message": "Which?", "fields": [] }),
        ),
    ]);
    assert_eq!(kinds(&timeline), ["prompt", "question"]);
    assert_eq!(timeline.attention(), Some(EventSeq::new(1)));
    assert_eq!(timeline.turn(), Some(EventSeq::new(0)));
    assert_eq!(timeline.waiting_questions().len(), 1);
    let prompt = timeline.entries()[0].clone();
    let change = timeline.apply(&[log.agent(&json!({
        "event": "question_resolved", "id": uuid(1), "outcome": { "outcome": "declined" }
    }))]);
    assert_eq!(timeline.entries().len(), 2);
    assert!(matches!(
        &timeline.entries()[1].kind,
        EntryKind::Question {
            outcome: Some(vessel_proto::QuestionOutcome::Declined),
            ..
        }
    ));
    assert_eq!(timeline.entries()[0], prompt);
    assert_eq!(change.changed, [1]);
    assert!(timeline.waiting_questions().is_empty());
}

#[test]
fn the_turn_stop_names_and_what_asks_for_attention() {
    let mut log = Log::default();
    let mut timeline = folded(&[
        log.agent(&json!({ "event": "turn_started", "content": [] })),
        log.agent(&json!({ "event": "message_chunk", "content": { "type": "text", "text": "x" } })),
        log.agent(&json!({ "event": "turn_ended", "reason": "end_turn" })),
    ]);
    assert_eq!(timeline.turn(), None);
    assert_eq!(timeline.attention(), Some(EventSeq::new(2)));
    let _change = timeline.apply(&[
        log.agent(&json!({ "event": "turn_started", "content": [] })),
        log.agent(&json!({ "event": "turn_failed", "message": "no" })),
    ]);
    assert_eq!(timeline.attention(), Some(EventSeq::new(4)));
    assert_eq!(timeline.turn(), None);
}

/// The records a real node produced for a `fake-acp` turn with its
/// permission answered: the page's fixture, read where it lies.
fn recorded() -> Vec<EventRecord> {
    serde_json::from_str(include_str!(
        "../../../../../web/src/data/fixtures/fake-acp-turn.json"
    ))
    .expect("every recorded record reads as one")
}

#[test]
fn folds_a_whole_recorded_turn() {
    let records = recorded();
    let timeline = folded(&records);
    assert_eq!(
        kinds(&timeline),
        [
            "status",
            "log",
            "ready",
            "status",
            "prompt",
            "thought",
            "message",
            "tool",
            "plan",
            "permission",
            "turn_ended"
        ]
    );
    let tool = timeline
        .entries()
        .iter()
        .find_map(|entry| entry.kind.as_tool())
        .unwrap();
    assert_eq!(tool.status, ToolCallStatus::Completed);
    assert!(timeline.pending().is_none());
    assert!(!timeline.in_turn());
    assert_eq!(timeline.turns(), 1);
    assert_eq!(
        timeline.next(),
        EventSeq::new(u64::try_from(records.len()).unwrap())
    );
    assert_eq!(
        timeline.entries()[0].at.to_rfc3339(),
        "2026-11-07T16:19:13.042183629+00:00"
    );
}

#[test]
fn gives_the_same_timeline_however_the_records_are_batched() {
    let records = recorded();
    let whole = folded(&records);
    let mut piecewise = Timeline::default();
    for record in &records {
        let _change = piecewise.apply(std::slice::from_ref(record));
    }
    assert_eq!(piecewise.entries(), whole.entries());
    assert_eq!(piecewise.plan(), whole.plan());
}
