//! A failed turn's words as one sentence (`timeline::failure`): the
//! 0.0.0.0.15 smoke's usage limit, other failures, and the reply that
//! only repeats the failure.

use chrono::DateTime;
use vessel_proto::{EventSeq, StopReason};

use crate::Lang;
use crate::timeline::{
    Entry, EntryKey, EntryKind, FailureKind, Prompt, repeats_failure, turn_failure,
};

/// The message the node wrote for Claude's 5-hour limit (the smoke's).
const LIMIT: &str = "Internal error: You've hit your 5-hour limit · resets 6:40am (UTC): {\n  \"errorKind\": \"rate_limit\"\n}";
const SENTENCE: &str = "You've hit your 5-hour limit · resets 6:40am (UTC)";

fn entry(seq: u64, kind: EntryKind) -> Entry {
    Entry {
        key: EntryKey::Record(EventSeq::new(seq)),
        seq: EventSeq::new(seq),
        through: EventSeq::new(seq.saturating_add(1)),
        extra_sources: crate::timeline::ExtraSources::default(),
        at: DateTime::UNIX_EPOCH,
        kind,
        rev: 0,
    }
}

fn prompt(seq: u64, text: &str) -> Entry {
    entry(
        seq,
        EntryKind::Prompt(Prompt {
            text: text.to_owned(),
            replayed: false,
            turn: 1,
            attachments: Vec::new(),
        }),
    )
}

fn reply(seq: u64, text: &str) -> Entry {
    entry(
        seq,
        EntryKind::Message {
            text: text.to_owned(),
        },
    )
}

fn failed(seq: u64, message: &str) -> Entry {
    entry(
        seq,
        EntryKind::TurnFailed {
            message: message.to_owned(),
        },
    )
}

#[test]
fn a_usage_limit_is_its_sentence_alone_without_the_wrapper_or_the_json() {
    let failure = turn_failure(LIMIT);
    assert_eq!(failure.sentence, SENTENCE);
    assert_eq!(failure.kind, FailureKind::Limit);
    for lang in [Lang::En, Lang::Ko] {
        let line = lang.conversation().spoken(&failed(4, LIMIT), catalog(lang));
        assert_eq!(line, SENTENCE, "{lang:?}");
    }
}

#[test]
fn another_failure_reads_turn_failed_and_its_sentence() {
    let failure = turn_failure("Invalid params: the model is not available: {\"code\": 400}");
    assert_eq!(failure.sentence, "the model is not available");
    assert_eq!(failure.kind, FailureKind::Other);
    assert_eq!(
        Lang::En
            .conversation()
            .spoken(&failed(4, "agent exited: signal 9"), catalog(Lang::En)),
        "Turn failed · agent exited: signal 9"
    );
    assert_eq!(
        Lang::Ko
            .conversation()
            .spoken(&failed(4, "agent exited: signal 9"), catalog(Lang::Ko)),
        "턴 실패 · agent exited: signal 9"
    );
}

// The fake agent's `fail` (and any agent whose error data is a string):
// the node writes `Internal error: "<words>"`.
#[test]
fn an_error_whose_data_is_a_string_reads_as_its_words() {
    let failure = turn_failure("Internal error: \"fake-acp: the turn failed\"");
    assert_eq!(failure.sentence, "fake-acp: the turn failed");
    assert_eq!(failure.kind, FailureKind::Other);
}

#[test]
fn a_limit_is_known_by_its_sentence_when_no_data_says_so() {
    assert_eq!(
        turn_failure("Claude AI usage limit reached").kind,
        FailureKind::Limit
    );
    assert_eq!(
        turn_failure("Internal error").sentence,
        "Internal error",
        "a bare generic message is kept rather than emptied"
    );
}

#[test]
fn a_reply_that_only_repeats_the_failure_is_held_back() {
    let entries = [
        prompt(1, "test"),
        reply(2, SENTENCE),
        failed(3, LIMIT),
        prompt(4, "again"),
        reply(5, "a real answer"),
        entry(
            6,
            EntryKind::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
                point: false,
            },
        ),
    ];
    assert!(repeats_failure(&entries, 1));
    assert!(!repeats_failure(&entries, 4), "a turn that ended normally");
    assert!(!repeats_failure(&entries, 0), "not a reply");
    let other = [
        prompt(1, "test"),
        reply(2, "partial work"),
        failed(3, LIMIT),
    ];
    assert!(
        !repeats_failure(&other, 1),
        "a reply with other words stays"
    );
}

fn catalog(lang: Lang) -> &'static crate::localization::Translator {
    crate::localization::Translator::get(
        crate::localization::Locale::from_tag(match lang {
            Lang::En => "en",
            Lang::Ko => "ko",
        })
        .unwrap(),
    )
    .unwrap()
}
