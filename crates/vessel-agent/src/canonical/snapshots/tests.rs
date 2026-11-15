//! Scoped enrichment may preserve evidence, never invent an association after reuse.
use super::*;
use serde_json::json;
use vessel_proto::{ToolCall, ToolCallUpdate};

fn call(id: &str, turn: Option<u64>, text: &str) -> AgentEvent {
    let scope = turn.map(|started| json!({"kind":"session_turn","started":started}));
    AgentEvent::ToolCall(serde_json::from_value(json!({"id":id,"title":"read","kind":"read","status":"pending","scope":scope,"detail":{"type":"read","file_path":text}})).unwrap())
}
fn update(id: &str, turn: Option<u64>) -> AgentEvent {
    let scope = turn.map(|started| json!({"kind":"session_turn","started":started}));
    AgentEvent::ToolCallUpdate(serde_json::from_value(json!({"id":id,"scope":scope,"content":[{"type":"content","content":{"type":"text","text":"result"}}]})).unwrap())
}
fn updated(event: AgentEvent) -> ToolCallUpdate {
    let AgentEvent::ToolCallUpdate(update) = event else {
        panic!("update")
    };
    update
}
fn announced(event: AgentEvent) -> ToolCall {
    let AgentEvent::ToolCall(call) = event else {
        panic!("call")
    };
    call
}
fn start(snapshots: &mut Snapshots) {
    snapshots.observe(&mut AgentEvent::TurnStarted {
        provenance: None,
        content: vec![],
        client_message_id: None,
    });
}

#[test]
fn explicit_late_scope_retains_its_arguments_without_changing_another_turn() {
    let mut snapshots = Snapshots::default();
    snapshots.observe(&mut call("same", Some(1), "a"));
    snapshots.observe(&mut call("same", Some(2), "b"));
    let mut late = update("same", Some(1));
    snapshots.observe(&mut late);
    let Some(ToolCallDetail::Read(detail)) = updated(late).detail.as_deref().cloned() else {
        panic!("read")
    };
    assert_eq!(detail.file_path.to_str(), Some("a"));
    assert_eq!(detail.content.as_deref(), Some("result"));
    let mut other = update("same", Some(2));
    snapshots.observe(&mut other);
    let Some(ToolCallDetail::Read(detail)) = updated(other).detail.as_deref().cloned() else {
        panic!("read")
    };
    assert_eq!(detail.file_path.to_str(), Some("b"));
}

#[test]
fn unscoped_old_arguments_are_not_guessed_in_a_new_turn_or_after_reuse() {
    let mut snapshots = Snapshots::default();
    start(&mut snapshots);
    snapshots.observe(&mut call("same", None, "a"));
    start(&mut snapshots);
    let mut late = update("same", None);
    snapshots.observe(&mut late);
    assert!(updated(late).detail.is_none());
    snapshots.observe(&mut call("same", None, "b"));
    let mut reused = update("same", None);
    snapshots.observe(&mut reused);
    assert!(updated(reused).detail.is_none());
}

#[test]
fn count_saturation_retains_old_evidence_and_never_evicts_to_admit_another() {
    let mut snapshots = Snapshots::default();
    for index in 0..MAX_DETAIL_SNAPSHOTS {
        snapshots.observe(&mut call(&format!("{index}"), Some(1), "a"));
    }
    let reserved = snapshots.bytes;
    snapshots.observe(&mut call("overflow", Some(1), "b"));
    assert_eq!(snapshots.entries.len(), MAX_DETAIL_SNAPSHOTS);
    assert_eq!(snapshots.bytes, reserved);
    let mut old = update("0", Some(1));
    snapshots.observe(&mut old);
    assert!(updated(old).detail.is_some());
    let mut refused = update("overflow", Some(1));
    snapshots.observe(&mut refused);
    assert!(updated(refused).detail.is_none());
}

#[test]
fn byte_and_body_bounds_do_not_remove_retained_keys() {
    let mut snapshots = Snapshots::default();
    let text = "a".repeat(MAX_DETAIL_BYTES.checked_div(2).unwrap());
    for index in 0..MAX_DETAIL_SNAPSHOTS {
        snapshots.observe(&mut call(&format!("{index}"), Some(1), &text));
    }
    assert!(snapshots.entries.len() < MAX_DETAIL_SNAPSHOTS);
    assert!(snapshots.bytes <= MAX_DETAIL_SNAPSHOT_BYTES);
    let before = (snapshots.entries.len(), snapshots.bytes);
    snapshots.observe(&mut call(
        "oversize",
        Some(1),
        &"a".repeat(MAX_DETAIL_BYTES),
    ));
    assert_eq!((snapshots.entries.len(), snapshots.bytes), before);
    assert!(snapshots.entries.contains_key(&(
        announced(call("0", Some(1), "a")).id,
        Some(Box::new(ToolCallScope::SessionTurn {
            started: vessel_proto::EventSeq::new(1)
        }))
    )));
}
