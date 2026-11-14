//! Forking a conversation (`timeline::fork`, protocol 14): which turn's
//! footer offers a fork, and a forked session's first line. Paseo's fork
//! menu sits on every finished turn's footer; ours on those whose end the
//! agent named (a point) and no rewind left behind.

use serde_json::json;
use vessel_app_view::Lang;
use vessel_app_view::timeline::{EntryKind, Timeline, fork_point};
use vessel_proto::{EventRecord, EventSeq, SessionId};

use crate::support::{Log, kinds};

fn turn(log: &mut Log, prompt: &str, point: Option<&str>) -> Vec<EventRecord> {
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
            &json!({ "event": "message_chunk", "content": { "type": "text", "text": "ok" } }),
        ),
        log.agent(&ended),
    ]
}

#[test]
fn a_turns_footer_offers_a_fork_where_the_agent_named_its_end() {
    let mut log = Log::default();
    let mut records = turn(&mut log, "first", Some("m1"));
    records.extend(turn(&mut log, "second", None));
    records.extend(turn(&mut log, "third", Some("m3")));
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&records);
    assert_eq!(fork_point(&timeline, 2), Some(EventSeq::new(2)));
    assert_eq!(fork_point(&timeline, 5), None, "no point");
    assert_eq!(fork_point(&timeline, 8), Some(EventSeq::new(8)));
    assert_eq!(fork_point(&timeline, 0), None, "a prompt");
    assert_eq!(fork_point(&timeline, 1), None, "a reply");
    assert_eq!(fork_point(&timeline, 9), None, "past the end");
}

#[test]
fn a_turn_a_rewind_left_behind_offers_no_fork() {
    let mut log = Log::default();
    let mut records = turn(&mut log, "first", Some("m1"));
    records.extend(turn(&mut log, "second", Some("m2")));
    records.push(log.record(
        &json!({ "type": "rewound", "after": 2, "harness_session": "second", "files_changed": 0 }),
    ));
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&records);
    assert_eq!(fork_point(&timeline, 2), Some(EventSeq::new(2)));
    assert_eq!(fork_point(&timeline, 5), None, "rewound away");
}

#[test]
fn a_forked_session_opens_with_a_line_naming_its_source() {
    let source = SessionId::random();
    let mut log = Log::default();
    let mut records = vec![log.record(&json!({
        "type": "forked", "from": source.to_string(), "after": 7
    }))];
    records.extend(turn(&mut log, "next", Some("m1")));
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&records);
    assert_eq!(kinds(&timeline).first(), Some(&"forked"));
    let EntryKind::Forked { from, after } = timeline.entries()[0].kind else {
        panic!("{:?}", timeline.entries()[0].kind);
    };
    assert_eq!((from, after), (source, Some(EventSeq::new(7))));
    let words = Lang::En.conversation();
    assert_eq!(
        words.forked_from(Some("plain")),
        "Forked from “plain”",
        "the source's title, from the list"
    );
    assert_eq!(words.forked_from(None), "Forked from another session");
    assert_eq!(
        Lang::Ko.conversation().forked_from(Some("plain")),
        "“plain”에서 분기함"
    );
}

#[test]
fn a_refused_fork_says_paseos_words_and_the_nodes() {
    let words = Lang::En.conversation();
    assert_eq!(words.fork_failed(" "), "Failed to fork chat");
    assert_eq!(
        words.fork_failed("the agent did not fork at the chosen point"),
        "Failed to fork chat: the agent did not fork at the chosen point"
    );
    assert_eq!(
        Lang::Ko.conversation().fork_failed(""),
        "채팅을 분기하지 못했습니다"
    );
}
