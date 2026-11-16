//! Earlier rows can receive later tool updates; revision counts are not cursors.

use super::Timeline;
use serde_json::{Value, json};
use vessel_proto::{EventRecord, EventSeq, SessionEvent};

fn record(seq: u64, event: Value) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: serde_json::from_value(event).unwrap(),
        },
    }
}

#[test]
fn a_tool_changed_after_the_selected_anchor_crosses_its_exclusive_bound() {
    let mut timeline = Timeline::default();
    let records = [
        record(
            2,
            json!({"event":"tool_call","id":"read","title":"Read","kind":"read","status":"in_progress"}),
        ),
        record(
            5,
            json!({"event":"message_chunk","content":{"type":"text","text":"Selected answer"}}),
        ),
    ];
    let _changed = timeline.apply(&records);
    let selected = timeline.entries()[1].through;
    assert_eq!(selected, EventSeq::new(6));
    assert_eq!(timeline.entries()[0].through, EventSeq::new(3));
    let update = record(
        9,
        json!({"event":"tool_call_update","id":"read","status":"completed"}),
    );
    let _changed = timeline.apply(std::slice::from_ref(&update));
    assert_eq!(timeline.entries()[0].seq, EventSeq::new(2));
    assert_eq!(timeline.entries()[0].through, EventSeq::new(10));
    assert!(timeline.entries()[0].through > selected);
    assert_eq!(timeline.entries()[1].through, selected);
    let _overlap = timeline.apply(&[update]);
    assert_eq!(timeline.entries()[0].through, EventSeq::new(10));
}

#[test]
fn merged_text_keeps_its_first_identity_and_last_contributing_record_bound() {
    let mut timeline = Timeline::default();
    let records = [2, 7].map(|seq| {
        record(
            seq,
            json!({"event":"message_chunk","content":{"type":"text","text":"part"}}),
        )
    });
    let _changed = timeline.apply(&records);
    assert_eq!(timeline.entries().len(), 1);
    let entry = &timeline.entries()[0];
    assert_eq!(entry.seq, EventSeq::new(2));
    assert_eq!(entry.through, EventSeq::new(8));
    assert_eq!(entry.rev, 1);
}
