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
