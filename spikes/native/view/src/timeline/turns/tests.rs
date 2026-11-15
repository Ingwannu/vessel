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
    tool_id("read")
}
fn tool_id(id: &str) -> Value {
    json!({"event":"tool_call","id":id,"title":"Read source","kind":"read","status":"completed"})
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

#[test]
fn row_projection_keeps_identity_and_reports_the_earliest_disappearing_row() {
    use crate::timeline::{Logs, Rows};
    use std::collections::HashSet;

    let timeline = timeline(vec![prompt(), tool(), reply("Done"), end()]);
    let entries = timeline.entries();
    let turns = TurnFolds::of(entries, TurnActivity::Closed);
    let rows = Rows::of(entries.len(), None, &Logs::default(), &HashSet::new());
    let closed = rows.clone().fold_turns(entries, &turns, &HashSet::new());
    assert_eq!(closed, Rows::Some(vec![0, 2, 3]));
    assert_eq!(rows.first_difference(&closed), 1);
    let opened = rows
        .clone()
        .fold_turns(entries, &turns, &HashSet::from([0]));
    assert_eq!(opened, Rows::Some(vec![0, 1, 2, 3]));
    assert_eq!(rows.first_difference(&opened), rows.len());
}

#[test]
fn a_human_steer_stays_visible_inside_the_folded_work() {
    use crate::timeline::{Logs, Rows};
    use std::collections::HashSet;

    let timeline = timeline(vec![
        prompt(),
        tool(),
        json!({"event":"steered","content":[{"type":"text","text":"Also test it"}]}),
        tool_id("read-again"),
        reply("Done"),
        end(),
    ]);
    let entries = timeline.entries();
    let turns = TurnFolds::of(entries, TurnActivity::Closed);
    let rows = Rows::of(entries.len(), None, &Logs::default(), &HashSet::new()).fold_turns(
        entries,
        &turns,
        &HashSet::new(),
    );
    assert_eq!(rows, Rows::Some(vec![0, 2, 4, 5]));
}

#[test]
fn automatically_expanded_reasoning_is_never_hidden_by_a_tool_group() {
    use crate::timeline::Folds;
    let timeline = timeline(vec![
        prompt(),
        tool(),
        json!({"event":"thought_chunk","content":{"type":"text","text":"Thinking"}}),
        tool_id("read-again"),
        reply("Done"),
        end(),
    ]);
    let ordinary = Folds::of(timeline.entries());
    assert!(ordinary.run_of(2).is_some());
    let expanded = Folds::with_reasoning(timeline.entries(), true);
    assert!(expanded.run_of(2).is_none());
    assert!(expanded.run_of(1).is_some());
    assert!(expanded.run_of(3).is_some());
}

#[test]
fn group_and_turn_summaries_count_distinct_reported_files() {
    use crate::timeline::Folds;
    let read = |id, path| json!({"event":"tool_call","id":id,"title":"Read","kind":"read","status":"completed","locations":[{"path":path}]});
    let timeline = timeline(vec![
        prompt(),
        read("first", "/work/a"),
        read("again", "/work/a"),
        read("other", "/work/b"),
        reply("Done"),
        end(),
    ]);
    assert_eq!(
        Folds::of(timeline.entries())
            .runs()
            .first()
            .unwrap()
            .summary
            .read,
        2
    );
    assert_eq!(
        TurnFolds::of(timeline.entries(), TurnActivity::Closed)
            .at_host(0)
            .unwrap()
            .summary
            .read,
        2
    );
}
