//! Forking a conversation (`timeline::fork`, protocol 14): which turn's
//! footer offers a fork, and a forked session's first line. Paseo's fork
//! menu sits on every finished turn's footer; ours on those whose end the
//! agent named (a point) and no rewind left behind.

use serde_json::json;
use vessel_app_view::Lang;
use vessel_app_view::timeline::{EntryKind, Timeline, Until, fork_point, origin_of};
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

#[test]
fn a_forks_origin_is_read_up_to_the_turn_it_continues_from() {
    let source = SessionId::random();
    let mut log = Log::default();
    let records = [
        log.record(&json!({ "type": "forked", "from": source.to_string(), "after": 5 })),
        log.record(&json!({ "type": "forked", "from": SessionId::random().to_string() })),
    ];
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&records[..1]);
    assert_eq!(
        origin_of(timeline.entries()),
        Some((source, Until::Record(EventSeq::new(5))))
    );
    let mut origin = Log::default();
    let read: Vec<EventRecord> = (0..8)
        .map(|_| {
            origin.agent(
                &json!({ "event": "message_chunk", "content": { "type": "text", "text": "x" } }),
            )
        })
        .collect();
    let kept: Vec<_> = read
        .iter()
        .filter(|record| Until::Record(EventSeq::new(5)).keeps(record))
        .map(|record| record.seq.get())
        .collect();
    assert_eq!(kept, [0, 1, 2, 3, 4, 5], "the turn's end itself is kept");

    // A whole-conversation fork names no record: what was made by the
    // time it was.
    let mut whole = Timeline::default();
    let _change = whole.apply(&records[1..]);
    let Some((_, Until::Time(made))) = origin_of(whole.entries()) else {
        panic!("{:?}", origin_of(whole.entries()));
    };
    assert_eq!(made, records[1].at);
    // The fixtures' records are all made at one moment: kept.
    assert!(Until::Time(made).keeps(&read[0]));
    let mut later = read[0].clone();
    later.at = made
        .checked_add_signed(chrono::TimeDelta::seconds(1))
        .expect("a moment later");
    assert!(!Until::Time(made).keeps(&later), "made after the fork");
    assert_eq!(origin_of(Timeline::default().entries()), None, "not a fork");
}

#[test]
fn an_origin_that_cannot_be_read_is_said_plainly() {
    let words = Lang::En.conversation();
    assert_eq!(
        words.origin_unread(true, "not found"),
        "The conversation it was forked from was removed."
    );
    assert_eq!(
        words.origin_unread(false, "connection lost."),
        "The conversation it was forked from could not be read: connection lost. Open the \
         session again to try again."
    );
    assert_eq!(
        Lang::Ko.conversation().origin_unread(false, " "),
        "분기한 원래 대화를 읽지 못했습니다. 세션을 다시 열면 다시 읽습니다."
    );
    assert_eq!(
        Lang::Ko.conversation().origin_unread(true, ""),
        "분기한 원래 대화는 삭제되었습니다."
    );
}

#[test]
fn an_empty_fork_says_why_in_the_persons_words() {
    use vessel_app_view::timeline::{EmptyFork, empty_fork};
    use vessel_proto::SessionStatus;
    let failed = |message: &str, empty_fork| SessionStatus::Failed {
        message: message.to_owned(),
        empty_fork,
    };
    assert_eq!(
        empty_fork(&failed("the agent did not fork at the chosen point", true)),
        Some(EmptyFork::NotAtPoint)
    );
    assert_eq!(
        empty_fork(&failed("the agent cannot fork a conversation", true)),
        Some(EmptyFork::CannotFork)
    );
    assert_eq!(
        empty_fork(&failed("`initialize` failed", true)),
        Some(EmptyFork::Other("`initialize` failed".to_owned()))
    );
    assert_eq!(
        empty_fork(&failed("the agent did not fork at the chosen point", false)),
        None,
        "another session's failure"
    );
    assert_eq!(empty_fork(&SessionStatus::Running), None);
    let words = Lang::Ko.conversation();
    assert_eq!(words.empty_fork_heading(), "분기가 시작되지 않았습니다");
    assert!(
        words
            .empty_fork_line(&EmptyFork::NotAtPoint)
            .starts_with("에이전트가 고른 지점에서 분기하지 않았습니다.")
    );
    assert_eq!(
        Lang::En
            .conversation()
            .empty_fork_line(&EmptyFork::Other("x".to_owned())),
        "x"
    );
    assert_eq!(Lang::En.conversation().remove_fork(), "Remove");
}
