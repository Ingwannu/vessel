use super::presented_prompt;
use crate::timeline::{
    Entry, EntryKey, EntryKind, ExtraSources, FindCache, Match, Prompt, find_all, searchable,
    spans_of,
};
use chrono::DateTime;
use vessel_proto::EventSeq;

const ENVELOPE: &str = "<spoken-input>\nPlease fix voice.\n</spoken-input>\n<instruction>This message was spoken by the user. Giraffe instruction.</instruction>";

#[test]
fn complete_current_and_legacy_envelopes_borrow_only_the_spoken_words() {
    assert_eq!(presented_prompt(ENVELOPE), "Please fix voice.");
    assert_eq!(
        presented_prompt(
            "<spoken-input>\nFirst.\nSecond with <example>XML</example>.\n</spoken-input>"
        ),
        "First.\nSecond with <example>XML</example>."
    );
    assert_eq!(presented_prompt("<spoken-input> \n </spoken-input>"), "");
}

#[test]
fn ordinary_quoted_incomplete_and_unrelated_instructions_remain_literal() {
    let quoted = format!("Explain this example: {ENVELOPE}");
    for text in [
        "ordinary message",
        quoted.as_str(),
        "<spoken-input>unfinished",
        "<spoken-input>Example</spoken-input><instruction>Explain this XML.</instruction>",
        "<spoken-input>Example</spoken-input><instruction>This message was spoken by the user.",
        "<spoken-input>Example</spoken-input> trailing user text",
    ] {
        assert_eq!(presented_prompt(text), text);
    }
}

#[test]
fn whitespace_and_embedded_closing_examples_match_the_anchored_reference() {
    assert_eq!(
        presented_prompt("\u{feff}<spoken-input>\u{a0}안녕\u{3000}</spoken-input>\u{2028}"),
        "안녕"
    );
    let non_ecmascript_space = "\u{85}<spoken-input>literal</spoken-input>";
    assert_eq!(presented_prompt(non_ecmascript_space), non_ecmascript_space);
    assert_eq!(
        presented_prompt("<spoken-input>Show </spoken-input> as text.</spoken-input>"),
        "Show </spoken-input> as text."
    );
}

fn entry(seq: u64, kind: EntryKind) -> Entry {
    Entry {
        key: EntryKey::Record(EventSeq::new(seq)),
        seq: EventSeq::new(seq),
        through: EventSeq::new(seq.saturating_add(1)),
        extra_sources: ExtraSources::default(),
        at: DateTime::UNIX_EPOCH,
        kind,
        rev: 0,
    }
}

#[test]
fn user_search_uses_display_offsets_but_assistant_examples_and_source_stay_whole() {
    let entries = [
        entry(
            1,
            EntryKind::Prompt(Prompt {
                text: ENVELOPE.to_owned(),
                replayed: false,
                turn: 1,
                attachments: Vec::new(),
            }),
        ),
        entry(
            2,
            EntryKind::Steer {
                text: ENVELOPE.to_owned(),
                attachments: Vec::new(),
            },
        ),
        entry(
            3,
            EntryKind::Message {
                text: ENVELOPE.to_owned(),
            },
        ),
    ];
    let first = entries.first().unwrap();
    let steer = entries.get(1).unwrap();
    let assistant = entries.get(2).unwrap();
    assert_eq!(searchable(first), Some("Please fix voice."));
    assert_eq!(searchable(steer), Some("Please fix voice."));
    assert_eq!(searchable(assistant), Some(ENVELOPE));
    assert_eq!(spans_of(searchable(first).unwrap(), "fix"), vec![7..10]);
    assert_eq!(
        find_all(&mut FindCache::default(), &entries, "giraffe"),
        vec![Match { entry: 2, nth: 0 }]
    );
    assert!(matches!(&first.kind, EntryKind::Prompt(prompt) if prompt.text == ENVELOPE));
    assert_eq!(first.key, EntryKey::Record(EventSeq::new(1)));
}
