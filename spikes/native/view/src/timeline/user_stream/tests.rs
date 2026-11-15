//! Real record ingestion preserves first user identity across diagnostic metadata.

use crate::timeline::{EntryKey, EntryKind, Match, Timeline, entry_at_source, start_at};
use chrono::DateTime;
use serde_json::json;
use vessel_proto::{EventRecord, EventSeq, SessionEvent};

fn chunk(seq: u64, text: &str) -> EventRecord {
    // Typed SessionEvent avoids depending on its external tag for agent records.
    EventRecord {
        seq: EventSeq::new(seq),
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: serde_json::from_value(
                json!({"event":"user_message_chunk","content":{"type":"text","text":text}}),
            )
            .unwrap(),
        },
    }
}

#[test]
fn replay_chunks_across_log_and_woken_metadata_keep_one_prompt_key_and_exact_through() {
    let mut timeline = Timeline::default();
    let first = chunk(1, "First ");
    let _change = timeline.apply(&[first]);
    let key = timeline.entries()[0].key.clone();
    let records = [
        EventRecord {
            seq: EventSeq::new(2),
            at: DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent {
                event: vessel_proto::AgentEvent::HarnessLog {
                    line: "diagnostic".to_owned(),
                },
            },
        },
        EventRecord {
            seq: EventSeq::new(3),
            at: DateTime::UNIX_EPOCH,
            event: SessionEvent::Woken,
        },
        chunk(8, "second"),
        EventRecord {
            seq: EventSeq::new(10), at: DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent {event: serde_json::from_value(json!({"event":"message_chunk","content":{"type":"text","text":"Other matching reply"}})).unwrap()},
        },
    ];
    let _change = timeline.apply(&records);
    assert_eq!(
        timeline
            .entries()
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::Prompt(_)))
            .count(),
        1
    );
    assert_eq!(timeline.entries()[0].key, key);
    assert_eq!(timeline.entries()[0].through, EventSeq::new(9));
    assert_eq!(
        entry_at_source(timeline.entries(), EventSeq::new(8)),
        Some(0)
    );
    assert_eq!(
        entry_at_source(timeline.entries(), EventSeq::new(2)),
        Some(1)
    );
    assert_eq!(
        start_at(
            timeline.entries(),
            timeline.next(),
            &[Match { entry: 0, nth: 0 }, Match { entry: 3, nth: 0 }],
            EventSeq::new(8)
        ),
        Some(0)
    );
    assert!(
        matches!(&timeline.entries()[0].kind,EntryKind::Prompt(prompt) if prompt.text=="First second")
    );
}

#[test]
fn overlapping_late_tool_bounds_do_not_claim_an_unrelated_human_source_record() {
    let mut timeline = Timeline::default();
    let tool = EventRecord {seq:EventSeq::new(1),at:DateTime::UNIX_EPOCH,event:SessionEvent::Agent {event:serde_json::from_value(json!({"event":"tool_call","id":"old","title":"Read","kind":"read","status":"in_progress"})).unwrap()}};
    let update = EventRecord {
        seq: EventSeq::new(9),
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: serde_json::from_value(
                json!({"event":"tool_call_update","id":"old","status":"completed"}),
            )
            .unwrap(),
        },
    };
    let _change = timeline.apply(&[tool, chunk(3, "first "), chunk(7, "part"), update]);
    assert_eq!(timeline.entries()[0].through, EventSeq::new(10));
    assert_eq!(timeline.entries()[1].through, EventSeq::new(8));
    assert_eq!(
        entry_at_source(timeline.entries(), EventSeq::new(7)),
        Some(1)
    );
    assert_eq!(
        entry_at_source(timeline.entries(), EventSeq::new(9)),
        Some(0)
    );
    assert_eq!(entry_at_source(timeline.entries(), EventSeq::new(5)), None);
}

#[test]
fn a_steer_is_extended_without_becoming_a_new_prompt_or_losing_its_record_key() {
    let mut timeline = Timeline::default();
    let steered = EventRecord {
        seq: EventSeq::new(1),
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: serde_json::from_value(
                json!({"event":"steered","content":[{"type":"text","text":"Steer "}]}),
            )
            .unwrap(),
        },
    };
    let _change = timeline.apply(&[steered, chunk(4, "continued")]);
    assert_eq!(timeline.entries().len(), 1);
    assert_eq!(
        timeline.entries()[0].key,
        EntryKey::Record(EventSeq::new(1))
    );
    assert_eq!(timeline.entries()[0].through, EventSeq::new(5));
    assert!(
        matches!(&timeline.entries()[0].kind,EntryKind::Steer{text,..} if text=="Steer continued")
    );
}

#[test]
fn real_agent_content_and_status_boundaries_start_a_new_replayed_prompt() {
    let mut timeline = Timeline::default();
    let content = EventRecord {
        seq: EventSeq::new(2),
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: serde_json::from_value(
                json!({"event":"message_chunk","content":{"type":"text","text":"Answer"}}),
            )
            .unwrap(),
        },
    };
    let _change = timeline.apply(&[chunk(1, "First"), content, chunk(3, "Second")]);
    assert_eq!(
        timeline
            .entries()
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::Prompt(_)))
            .count(),
        2
    );
    assert!(super::preserves(&SessionEvent::Woken));
    assert!(!super::preserves(&SessionEvent::Status {
        status: vessel_proto::SessionStatus::Running
    }));
}

#[test]
fn a_terminal_boundary_flushes_even_though_it_adds_no_structured_entry() {
    let mut timeline = Timeline::default();
    let output = EventRecord {
        seq: EventSeq::new(2),
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Terminal(serde_json::from_value(json!({"data":"eA=="})).unwrap()),
    };
    let _change = timeline.apply(&[chunk(1, "First"), output, chunk(3, "Second")]);
    assert_eq!(timeline.entries().len(), 2);
    assert!(
        matches!(&timeline.entries()[0].kind, EntryKind::Prompt(prompt) if prompt.text == "First")
    );
    assert!(
        matches!(&timeline.entries()[1].kind, EntryKind::Prompt(prompt) if prompt.text == "Second")
    );
}
