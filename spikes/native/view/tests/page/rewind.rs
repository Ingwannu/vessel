//! Rewinding a conversation (`timeline::rewind`, protocol 14): where a
//! person's message can be rewound to, and what a rewind leaves behind.
//! Paseo's rewind menu sits on every user message the agent can rewind
//! to; ours on those whose turn before ended at a point.

use serde_json::json;
use vessel_app_view::timeline::{Timeline, rewind_point};
use vessel_proto::{EventRecord, EventSeq};

use crate::support::Log;

fn turn(log: &mut Log, prompt: &str, reply: &str, point: Option<&str>) -> Vec<EventRecord> {
    let ended = point.map_or_else(
        || json!({ "event": "turn_ended", "reason": "end_turn" }),
        |message| {
            json!({ "event": "turn_ended", "reason": "end_turn",
                "point": { "message": message, "conversation": "first" } })
        },
    );
    vec![
        log.agent(
            &json!({ "event": "turn_started", "content": [{ "type": "text", "text": prompt }] }),
        ),
        log.agent(
            &json!({ "event": "message_chunk", "content": { "type": "text", "text": reply } }),
        ),
        log.agent(&ended),
    ]
}

/// Three turns, the last without a point, then a fourth prompt: entries
/// 0..=2, 3..=5, 6..=8, 9.
fn three_turns() -> (Timeline, Log) {
    let mut log = Log::default();
    let mut records = turn(&mut log, "first", "one", Some("m1"));
    records.extend(turn(&mut log, "second", "two", Some("m2")));
    records.extend(turn(&mut log, "third", "three", None));
    records.push(log.agent(
        &json!({ "event": "turn_started", "content": [{ "type": "text", "text": "fourth" }] }),
    ));
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&records);
    (timeline, log)
}

#[test]
fn a_message_rewinds_to_the_turn_end_before_it_when_the_agent_named_it() {
    let (timeline, _log) = three_turns();
    assert_eq!(timeline.entries().len(), 10);
    assert_eq!(rewind_point(&timeline, 0), None, "the first message");
    assert_eq!(rewind_point(&timeline, 1), None, "a reply");
    assert_eq!(rewind_point(&timeline, 3), Some(EventSeq::new(2)));
    assert_eq!(rewind_point(&timeline, 6), Some(EventSeq::new(5)));
    assert_eq!(
        rewind_point(&timeline, 9),
        None,
        "the turn before ended without a point"
    );
}

#[test]
fn a_rewind_leaves_what_followed_behind_and_the_next_message_rewinds_past_it() {
    let (mut timeline, mut log) = three_turns();
    let rewound = log.record(
        &json!({ "type": "rewound", "after": 2, "harness_session": "second", "files_changed": 0 }),
    );
    let change = timeline.apply(&[rewound]);
    assert_eq!(change.changed, (3..10).collect::<Vec<_>>(), "redrawn");
    assert!(!timeline.rewound(2));
    assert!((3..10).all(|index| timeline.rewound(index)));
    let again = log.agent(
        &json!({ "event": "turn_started", "content": [{ "type": "text", "text": "again" }] }),
    );
    let _change = timeline.apply(&[again]);
    assert!(!timeline.rewound(10), "what came after the rewind is shown");
    assert_eq!(
        rewind_point(&timeline, 10),
        Some(EventSeq::new(2)),
        "past the turns the rewind left behind"
    );
}
