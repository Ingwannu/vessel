//! The palette's message search across machines (`search.rs`): the rows
//! newest first, a row keeping its place when a slower machine answers,
//! a machine's older hits held back until its next page, and what the
//! group says of a machine too old, one refusing, and no match.

use vessel_proto::EventSeq;
use vessel_proto::search::{
    MessageRole, SearchCursor, SearchHit, SearchPage, Snippet, SnippetPart,
};

use super::fixture::{at, id};
use crate::search::{Answer, HitId, MachineState, MessageSearch, Note, PAGE, Row};

fn hit(session: &str, seq: u64, when: &str) -> SearchHit {
    SearchHit {
        session: id(session),
        seq: EventSeq::new(seq),
        at: at(when),
        role: MessageRole::Reply,
        snippet: Snippet {
            parts: vec![SnippetPart {
                text: "fold".to_owned(),
                matched: true,
            }],
            cut_before: false,
            cut_after: false,
        },
    }
}

fn page(hits: Vec<SearchHit>) -> Answer {
    Answer::Page(SearchPage {
        hits,
        next: None,
        reached: None,
    })
}

fn row(machine: &str, session: &str, seq: u64) -> Row {
    Row::Hit(HitId {
        machine: machine.to_owned(),
        session: id(session),
        seq: EventSeq::new(seq),
    })
}

#[test]
fn every_machine_is_asked_and_a_later_answer_goes_below_the_rows_shown() {
    let mut search = MessageSearch::default();
    let asks = search.start("fold", ["laptop", "build"]);
    assert_eq!(
        asks.iter()
            .map(|ask| ask.machine.as_str())
            .collect::<Vec<_>>(),
        ["laptop", "build"]
    );
    assert_eq!(search.notes(), [Note::Searching]);
    let ticket = search.ticket();
    let more = search.answered(
        ticket,
        "laptop",
        page(vec![
            hit("a2", 4, "2026-11-14T08:00:00Z"),
            hit("a1", 9, "2026-11-14T10:00:00Z"),
        ]),
    );
    assert!(more.is_empty());
    // One machine's hits, newest first.
    assert_eq!(
        search.rows(),
        [row("laptop", "a1", 9), row("laptop", "a2", 4)]
    );
    // A machine answering later adds its rows below the ones shown (GOAT:
    // a row does not move under the person's pointer), even a newer one.
    let _none = search.answered(
        ticket,
        "build",
        page(vec![hit("b1", 7, "2026-11-14T09:00:00Z")]),
    );
    assert_eq!(
        search.rows(),
        [
            row("laptop", "a1", 9),
            row("laptop", "a2", 4),
            row("build", "b1", 7)
        ]
    );
    assert!(search.notes().is_empty());
}

#[test]
fn a_row_shown_keeps_its_place_when_a_slower_machine_answers_newer() {
    let mut search = MessageSearch::default();
    let _asks = search.start("fold", ["laptop", "build"]);
    let ticket = search.ticket();
    let _none = search.answered(
        ticket,
        "laptop",
        page(vec![hit("a1", 3, "2026-11-14T08:00:00Z")]),
    );
    let _none = search.answered(
        ticket,
        "build",
        page(vec![hit("b1", 5, "2026-11-14T12:00:00Z")]),
    );
    assert_eq!(
        search.rows(),
        [row("laptop", "a1", 3), row("build", "b1", 5)]
    );
}

#[test]
fn a_machines_older_hits_wait_for_its_next_page() {
    let mut search = MessageSearch::default();
    let _asks = search.start("fold", ["laptop", "build"]);
    let ticket = search.ticket();
    let _none = search.answered(
        ticket,
        "laptop",
        page(vec![hit("a1", 2, "2026-11-14T09:00:00Z")]),
    );
    // build fills the group with ten hits from 12:00 on and read back only
    // to 11:00: laptop's 09:00 hit may be outranked by build's next page,
    // so it waits behind More, and nothing is asked while the group is
    // full.
    let full = (0..10)
        .map(|minute| hit("b1", 20 + minute, &format!("2026-11-14T12:{minute:02}:00Z")))
        .collect();
    let asks = search.answered(
        ticket,
        "build",
        Answer::Page(SearchPage {
            hits: full,
            next: Some(SearchCursor::new(41)),
            reached: Some(at("2026-11-14T11:00:00Z")),
        }),
    );
    assert!(asks.is_empty());
    let rows = search.rows();
    assert_eq!(rows.len(), PAGE + 1);
    assert!(!rows.contains(&row("laptop", "a1", 2)));
    assert_eq!(rows.last(), Some(&Row::More));
    // More asks build for its next page from where it stopped.
    let asks = search.more();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks.first().map(|ask| ask.machine.as_str()), Some("build"));
    assert_eq!(
        asks.first().and_then(|ask| ask.request.before),
        Some(SearchCursor::new(41))
    );
    // Its last page holds nothing newer: laptop's hit shows after build's.
    let _none = search.answered(ticket, "build", page(Vec::new()));
    assert_eq!(search.rows().last(), Some(&row("laptop", "a1", 2)));
}

#[test]
fn more_rows_are_shown_ten_at_a_time() {
    let mut search = MessageSearch::default();
    let _asks = search.start("fold", ["laptop"]);
    let ticket = search.ticket();
    let hits = (1..=15)
        .map(|seq| hit("a1", seq, &format!("2026-11-14T{seq:02}:00:00Z")))
        .collect();
    let _none = search.answered(ticket, "laptop", page(hits));
    let rows = search.rows();
    assert_eq!(rows.len(), PAGE + 1);
    assert_eq!(rows.last(), Some(&Row::More));
    let asks = search.more();
    assert!(asks.is_empty());
    assert_eq!(search.rows().len(), 15);
    assert!(!search.rows().contains(&Row::More));
}

#[test]
fn an_answer_to_an_earlier_search_is_dropped() {
    let mut search = MessageSearch::default();
    let _asks = search.start("fol", ["laptop"]);
    let earlier = search.ticket();
    let _asks = search.start("fold", ["laptop"]);
    let _none = search.answered(
        earlier,
        "laptop",
        page(vec![hit("a1", 1, "2026-11-14T08:00:00Z")]),
    );
    assert!(search.rows().is_empty());
    assert_eq!(search.query(), Some("fold"));
}

#[test]
fn an_old_machine_and_a_refusal_are_said_and_no_match_is_said_once_all_answered() {
    let mut search = MessageSearch::default();
    let _asks = search.start("fold", ["laptop", "old", "locked"]);
    let ticket = search.ticket();
    let _none = search.answered(ticket, "laptop", page(Vec::new()));
    let _none = search.answered(ticket, "old", Answer::Older);
    assert_eq!(search.notes().first(), Some(&Note::Searching));
    let _none = search.answered(ticket, "locked", Answer::Failed("not allowed".to_owned()));
    assert_eq!(
        search.notes(),
        [
            Note::Nothing,
            Note::Older(vec!["old".to_owned()]),
            Note::Failed(vec![("locked".to_owned(), "not allowed".to_owned())]),
        ]
    );
    assert_eq!(
        search.states().map(|(name, _)| name).collect::<Vec<_>>(),
        ["laptop", "old", "locked"]
    );
    assert!(matches!(
        search.states().nth(1),
        Some((_, MachineState::Older))
    ));
}

#[test]
fn words_the_node_would_refuse_ask_nothing_and_say_why_and_empty_words_end_the_search() {
    let mut search = MessageSearch::default();
    let long = "x".repeat(300);
    assert!(search.start(&long, ["laptop"]).is_empty());
    assert_eq!(search.notes(), [Note::Invalid]);
    assert!(search.active());
    assert!(search.start("   ", ["laptop"]).is_empty());
    assert!(!search.active());
    assert!(search.notes().is_empty());
}
