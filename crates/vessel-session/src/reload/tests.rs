//! The log's prompts and what a reload adds (TRACKER 803, ruling C "R2"):
//! nothing new, turns made outside Vessel added once, a history that does
//! not begin with the log's prompts adding nothing, and the bounds.

use vessel_proto::{AgentEvent, ContentBlock, SessionEvent, SessionStatus, StopReason};

use super::UserPrompts;
use super::rehydrate::{
    APPEND_MAX, DIVERGED, GATHER_ITEMS, GATHER_TIME, Gather, MORE, Outcome, UNKNOWN, outcome,
};

fn text(text: &str) -> ContentBlock {
    ContentBlock::Text {
        text: text.to_owned(),
    }
}

fn agent(event: AgentEvent) -> SessionEvent {
    SessionEvent::Agent { event }
}

fn user(chunk: &str) -> AgentEvent {
    AgentEvent::UserMessageChunk {
        content: text(chunk),
    }
}

fn said(chunk: &str) -> AgentEvent {
    AgentEvent::MessageChunk {
        content: text(chunk),
    }
}

fn ended() -> AgentEvent {
    AgentEvent::TurnEnded {
        reason: StopReason::EndTurn,
        usage: None,
        point: None,
    }
}

/// A log of one turn per prompt, as Vessel records a prompt it sent.
fn log_of(prompts: &[&str]) -> UserPrompts {
    let mut log = UserPrompts::new();
    for prompt in prompts {
        log.observe(&agent(AgentEvent::TurnStarted {
            content: vec![text(prompt)],
        }));
        log.observe(&agent(said("answer")));
        log.observe(&agent(ended()));
    }
    log
}

/// An agent's history: each prompt, then its answer.
fn history(prompts: &[&str]) -> Gather {
    let mut gather = Gather::new();
    for prompt in prompts {
        gather.push(user(prompt));
        gather.push(said(&format!("answer to {prompt}")));
    }
    gather
}

#[test]
fn a_turn_a_steer_and_a_run_of_chunks_outside_a_turn_are_prompts_and_an_echo_is_not() {
    let mut log = UserPrompts::new();
    log.observe(&agent(AgentEvent::TurnStarted {
        content: vec![text("look at "), text("this")],
    }));
    // The agent echoing the prompt inside its turn.
    log.observe(&agent(user("look at this")));
    log.observe(&agent(AgentEvent::Steered {
        content: vec![text("and that")],
    }));
    log.observe(&agent(ended()));
    // An import's (or a reload's) records: one message in two chunks, a
    // record of the node's between them.
    log.observe(&agent(user("made ")));
    log.observe(&SessionEvent::OptionsChanged {
        options: Vec::new(),
    });
    log.observe(&agent(user("outside")));
    log.observe(&agent(said("ok")));
    log.observe(&agent(user("still open")));
    assert_eq!(
        log.prompts().unwrap(),
        ["look at this", "and that", "made outside", "still open"]
    );
    log.observe(&SessionEvent::Status {
        status: SessionStatus::Exited { code: Some(0) },
    });
    assert_eq!(log.prompts().unwrap().len(), 4);
}

#[test]
fn a_fold_joins_the_one_before_its_run_and_knows_nothing_past_its_bound() {
    let joined = log_of(&["one"]).then(&log_of(&["two"]));
    assert_eq!(joined.prompts().unwrap(), ["one", "two"]);
    assert_eq!(
        UserPrompts::unknown().then(&log_of(&["two"])).prompts(),
        None
    );
    let mut big = UserPrompts::new();
    big.observe(&agent(AgentEvent::TurnStarted {
        content: vec![text(&"x".repeat(UserPrompts::MAX_BYTES))],
    }));
    assert!(big.prompts().is_some(), "at the bound");
    big.observe(&agent(AgentEvent::Steered {
        content: vec![text("y")],
    }));
    assert_eq!(big.prompts(), None, "past it");
}

#[test]
fn a_history_with_nothing_new_adds_nothing() {
    let log = log_of(&["plain"]);
    assert_eq!(outcome(&log, history(&["plain"])), Outcome::Current);
    assert_eq!(
        outcome(&UserPrompts::new(), Gather::new()),
        Outcome::Current
    );
}

#[test]
fn turns_made_outside_vessel_are_added_once() {
    let log = log_of(&["plain"]);
    let Outcome::Append { records, more } = outcome(&log, history(&["plain", "outside"])) else {
        panic!("the turn made outside is added");
    };
    assert!(!more);
    assert_eq!(records, [user("outside"), said("answer to outside")]);
    // Once added, the log's prompts are the history's: a second reload
    // adds nothing.
    let mut after = log;
    for record in records {
        after.observe(&agent(record));
    }
    assert_eq!(after.prompts().unwrap(), ["plain", "outside"]);
    assert_eq!(
        outcome(&after, history(&["plain", "outside"])),
        Outcome::Current
    );
}

#[test]
fn a_history_that_does_not_begin_with_the_logs_prompts_adds_nothing() {
    let log = log_of(&["plain"]);
    for other in [
        history(&["earlier question"]),
        history(&["earlier question", "plain"]),
        history(&[]),
        // The same words, cut differently: compared exactly.
        history(&["plain "]),
    ] {
        assert_eq!(outcome(&log, other), Outcome::Nothing(DIVERGED));
    }
    assert_eq!(
        outcome(&UserPrompts::unknown(), history(&["plain"])),
        Outcome::Nothing(UNKNOWN)
    );
}

#[test]
fn what_is_added_is_whole_messages_under_the_cap() {
    let log = log_of(&["plain"]);
    let mut gather = history(&["plain"]);
    // Two messages beyond the log: the first fits, the second does not.
    let half = APPEND_MAX.checked_div(2).unwrap();
    for name in ["first", "second"] {
        gather.push(user(name));
        for _ in 0..half {
            gather.push(said("."));
        }
    }
    let Outcome::Append { records, more } = outcome(&log, gather) else {
        panic!("the first message fits");
    };
    assert!(more, "the second waits for another reload");
    assert_eq!(records.len(), half + 1);
    assert_eq!(records.first(), Some(&user("first")));
    assert!(MORE.starts_with("vessel: "));
    // One message longer than the cap adds nothing.
    let mut oversized = history(&["plain"]);
    oversized.push(user("long"));
    for _ in 0..APPEND_MAX {
        oversized.push(said("."));
    }
    assert!(matches!(outcome(&log, oversized), Outcome::Nothing(_)));
}

#[test]
fn a_gather_past_its_items_adds_nothing() {
    let mut gather = Gather::new();
    for _ in 0..=GATHER_ITEMS {
        gather.push(said("."));
    }
    assert!(matches!(
        outcome(&UserPrompts::new(), gather),
        Outcome::Nothing(line) if line.contains("20000")
    ));
}

#[tokio::test(start_paused = true)]
async fn a_gather_past_its_time_adds_nothing() {
    let mut gather = Gather::new();
    gather.push(user("plain"));
    tokio::time::advance(GATHER_TIME).await;
    tokio::time::advance(std::time::Duration::from_millis(1)).await;
    gather.push(said("late"));
    assert!(matches!(
        outcome(&log_of(&["plain"]), gather),
        Outcome::Nothing(line) if line.contains("60 seconds")
    ));
}
