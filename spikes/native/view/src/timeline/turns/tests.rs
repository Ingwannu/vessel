//! A visible prompt and final answer are hard boundaries, including partial history.

use super::*;
use crate::timeline::Timeline;
use serde_json::{Value, json};
use vessel_proto::{EventRecord, EventSeq, SessionEvent};

fn timeline(events: Vec<Value>) -> Timeline {
    let records: Vec<EventRecord> = events
        .into_iter()
        .enumerate()
        .map(|(seq, event)| EventRecord {
            seq: EventSeq::new(u64::try_from(seq).unwrap()),
            at: chrono::DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent {
                event: serde_json::from_value(event).unwrap(),
            },
        })
        .collect();
    let mut timeline = Timeline::default();
    let _changed = timeline.apply(&records);
    timeline
}

fn prompt() -> Value {
    json!({"event":"turn_started","content":[{"type":"text","text":"Go"}]})
}
fn reply(text: &str) -> Value {
    json!({"event":"message_chunk","content":{"type":"text","text":text}})
}
fn tool() -> Value {
    json!({"event":"tool_call","id":"read","title":"Read source","kind":"read","status":"completed"})
}
fn end() -> Value {
    json!({"event":"turn_ended","reason":"end_turn"})
}

#[test]
fn a_closed_turn_folds_work_but_keeps_trigger_final_reply_and_footer() {
    let timeline = timeline(vec![prompt(), tool(), reply("Done"), end()]);
    let folds = TurnFolds::of(timeline.entries(), TurnActivity::Closed);
    let turn = folds.at_host(0).unwrap();
    assert_eq!(turn.hidden, 1..2);
    assert_eq!(turn.summary.read, 1);
    assert!(folds.containing(0).is_none());
    assert_eq!(folds.containing(1).unwrap().host, 0);
    assert!(folds.containing(2).is_none());
    assert!(folds.containing(3).is_none());
}

#[test]
fn the_open_latest_turn_and_reasoning_without_work_stay_unfolded() {
    let open = timeline(vec![prompt(), tool(), reply("Still working")]);
    assert!(
        TurnFolds::of(open.entries(), TurnActivity::Open)
            .at_host(0)
            .is_none()
    );
    let only_reasoning = timeline(vec![
        prompt(),
        json!({"event":"thought_chunk","content":{"type":"text","text":"Thinking"}}),
        reply("Answer"),
        end(),
    ]);
    assert!(
        TurnFolds::of(only_reasoning.entries(), TurnActivity::Closed)
            .at_host(0)
            .is_none()
    );
}

#[test]
fn previous_assistant_prose_folds_and_a_partial_history_never_gets_a_fake_host() {
    let full = timeline(vec![
        prompt(),
        reply("First update"),
        tool(),
        reply("Final"),
        end(),
    ]);
    assert_eq!(
        TurnFolds::of(full.entries(), TurnActivity::Closed)
            .at_host(0)
            .unwrap()
            .hidden,
        1..3
    );
    let partial = timeline(vec![tool(), reply("Answer"), end()]);
    assert!(
        TurnFolds::of(partial.entries(), TurnActivity::Closed)
            .at_host(0)
            .is_none()
    );
}
