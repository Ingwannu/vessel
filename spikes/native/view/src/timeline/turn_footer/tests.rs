//! Partial histories and resumed turns must not borrow a previous response's start.

use super::*;
use crate::timeline::Timeline;
use serde_json::{Value, json};
use vessel_proto::{EventRecord, EventSeq, SessionEvent};

fn entries(events: Vec<Value>) -> Timeline {
    let records: Vec<EventRecord> = events
        .into_iter()
        .enumerate()
        .map(|(seq, event)| EventRecord {
            seq: EventSeq::new(u64::try_from(seq).unwrap()),
            at: DateTime::from_timestamp(i64::try_from(seq).unwrap(), 0).unwrap(),
            event: SessionEvent::Agent {
                event: serde_json::from_value(event).unwrap(),
            },
        })
        .collect();
    let mut timeline = Timeline::default();
    let _changed = timeline.apply(&records);
    timeline
}

fn message(text: &str) -> Value {
    json!({"event":"message_chunk","content":{"type":"text","text":text}})
}
fn end() -> Value {
    json!({"event":"turn_ended","reason":"end_turn"})
}

#[test]
fn partial_history_keeps_the_timestamp_without_inventing_a_start() {
    let timeline = entries(vec![message("Answer"), end()]);
    let footer = TurnFooter::of(timeline.entries(), 1).unwrap();
    assert_eq!(footer.started_at, None);
    assert_eq!(footer.completed_at, DateTime::from_timestamp(1, 0).unwrap());
    assert_eq!(footer.response, "Answer");
}

#[test]
fn a_new_response_never_copies_prose_or_duration_from_before_the_last_boundary() {
    let timeline = entries(vec![
        json!({"event":"turn_started","content":[]}),
        message("Old answer"),
        end(),
        message("New answer"),
        end(),
    ]);
    let footer = TurnFooter::of(timeline.entries(), 4).unwrap();
    assert_eq!(footer.started_at, None);
    assert_eq!(footer.response, "New answer");
    assert!(TurnFooter::of(timeline.entries(), usize::MAX).is_none());
}

#[test]
fn duration_thresholds_do_not_drift() {
    let start = DateTime::UNIX_EPOCH;
    for (seconds, label) in [
        (-1, "0s"),
        (59, "59s"),
        (60, "1m"),
        (125, "2m 5s"),
        (3600, "1h"),
        (3780, "1h 3m"),
    ] {
        assert_eq!(
            duration_label(start, DateTime::from_timestamp(seconds, 0).unwrap()),
            label
        );
    }
}

#[test]
fn failed_responses_keep_their_footer_but_do_not_copy_the_withheld_error_reply() {
    let timeline = entries(vec![
        json!({"event":"turn_started","content":[]}),
        message("Partial work"),
        json!({"event":"tool_call","id":"read","title":"Read","kind":"read","status":"completed"}),
        message("service unavailable"),
        json!({"event":"turn_failed","message":"Internal error: service unavailable"}),
    ]);
    let footer = TurnFooter::of(timeline.entries(), 4).unwrap();
    assert_eq!(footer.completion, TurnCompletion::Failed);
    assert_eq!(footer.response, "Partial work");
    assert_eq!(footer.started_at, Some(DateTime::UNIX_EPOCH));
    let failed_only = entries(vec![json!({"event":"turn_failed","message":"unavailable"})]);
    assert!(TurnFooter::of(failed_only.entries(), 0).is_none());
}

#[test]
fn a_withheld_provider_error_alone_does_not_create_a_response_footer() {
    let timeline = entries(vec![
        json!({"event":"turn_started","content":[]}),
        message("service unavailable"),
        json!({"event":"turn_failed","message":"Internal error: service unavailable"}),
    ]);
    assert!(TurnFooter::of(timeline.entries(), 2).is_none());
}
