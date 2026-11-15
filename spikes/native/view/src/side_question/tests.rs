//! Drive the real reducer, including late replies and uncertain reset.
use super::*;
use crate::side_question::Question;
use vessel_proto::automation::AutomationText;

fn target() -> Target {
    Target {
        machine: MachineKey::from_book("scratch"),
        parent: SessionId::random(),
    }
}

fn ready() -> (State, SessionId) {
    let mut state = State::default();
    state.select(target());
    let open = state.begin(Operation::Open).unwrap();
    let side = SessionId::random();
    assert!(state.opened(&open, Ok(side)));
    (state, side)
}

#[test]
fn closed_busy_and_unopened_panels_issue_no_work() {
    let mut state = State::default();
    assert_eq!(state.begin(Operation::Open), Err(Problem::Closed));
    state.select(target());
    assert_eq!(state.begin(Operation::Prompt), Err(Problem::NoSide));
    let open = state.begin(Operation::Open).unwrap();
    assert_eq!(state.begin(Operation::Reset), Err(Problem::Busy));
    assert_eq!(state.pending(), Some(&open));
}

#[test]
fn navigation_and_close_do_not_accept_late_open_results() {
    let a = target();
    let mut state = State::default();
    state.select(a.clone());
    let old = state.begin(Operation::Open).unwrap();
    state.select(target());
    let other = state.begin(Operation::Open).unwrap();
    state.select(a.clone());
    let current = state.begin(Operation::Open).unwrap();
    assert!(!state.opened(&old, Ok(SessionId::random())));
    assert!(!state.opened(&other, Err(Problem::Offline)));
    assert_eq!(state.pending(), Some(&current));
    state.close();
    state.select(a);
    let reopened = state.begin(Operation::Open).unwrap();
    assert_ne!(current.ticket, reopened.ticket);
    assert!(!state.opened(&current, Ok(SessionId::random())));
    assert_eq!(state.side(), None);
}

#[test]
fn reset_failure_requires_discovery_not_automatic_replacement() {
    let (mut state, side) = ready();
    let reset = state.begin(Operation::Reset).unwrap();
    assert_eq!(state.side(), Some(side));
    assert!(state.opened(&reset, Err(Problem::Uncertain)));
    assert_eq!(state.side(), None);
    assert_eq!(state.problem(), Some(Problem::Uncertain));
    assert_eq!(state.begin(Operation::Reset), Err(Problem::NoSide));
    assert_eq!(state.begin(Operation::Prompt), Err(Problem::NoSide));
    let find = state.begin(Operation::Open).unwrap();
    let replacement = SessionId::random();
    assert!(state.opened(&find, Ok(replacement)));
    assert_eq!(state.side(), Some(replacement));
    assert_eq!(state.problem(), None);
    assert!(!state.opened(&reset, Ok(side)));
}

#[test]
fn wrong_reply_kind_cannot_settle_pending_work() {
    let (mut state, side) = ready();
    let send = state.begin(Operation::Prompt).unwrap();
    assert!(!state.opened(&send, Ok(SessionId::random())));
    assert_eq!(state.pending(), Some(&send));
    assert!(state.prompted(&send, Err(Problem::Unsent)));
    assert_eq!(state.side(), Some(side));
    assert_eq!(state.problem(), Some(Problem::Unsent));
    assert!(!state.prompted(&send, Ok(())));
    let reset = state.begin(Operation::Reset).unwrap();
    assert!(!state.prompted(&reset, Ok(())));
    assert_eq!(state.pending(), Some(&reset));
}

#[test]
fn parent_id_never_becomes_the_side_input_target() {
    let mut state = State::default();
    let parent = target();
    state.select(parent.clone());
    let open = state.begin(Operation::Open).unwrap();
    assert!(state.opened(&open, Ok(parent.parent)));
    assert_eq!(state.side(), None);
    assert_eq!(state.problem(), Some(Problem::InvalidReply));
}

#[test]
fn exhausted_ticket_source_preserves_the_previous_state() {
    let (mut state, side) = ready();
    state.next = RequestId::new(u64::MAX);
    assert_eq!(state.begin(Operation::Reset), Err(Problem::Exhausted));
    assert_eq!(state.side(), Some(side));
    assert_eq!(state.pending(), None);
}

#[test]
fn questions_share_the_wire_bound_and_never_interpret_slash_commands() {
    assert!(Question::parse("왜 이 작업을 하나요?\nExplain.").is_ok());
    for input in ["", " \n", "\u{feff}", "bad\0input"] {
        assert!(matches!(
            Question::parse(input),
            Err(Problem::InvalidQuestion(_))
        ));
    }
    for input in [
        "/side",
        " /model other",
        "\u{feff}\n/reset",
        "\u{2003}/clear",
    ] {
        assert_eq!(Question::parse(input), Err(Problem::Slash));
    }
    assert!(Question::parse(&"a".repeat(AutomationText::PROMPT_MAX)).is_ok());
    assert!(matches!(
        Question::parse(&"a".repeat(AutomationText::PROMPT_MAX.saturating_add(1))),
        Err(Problem::InvalidQuestion(_))
    ));
}

#[test]
fn valid_question_text_is_preserved_including_indentation() {
    let text = "  explain this\n    keep this indentation";
    assert_eq!(Question::parse(text).unwrap().prompt().as_str(), text);
}

#[test]
fn an_uncertain_delivery_requires_discovery_before_an_explicit_retry() {
    let (mut state, side) = ready();
    let send = state.begin(Operation::Prompt).unwrap();
    assert!(state.prompted(&send, Err(Problem::Uncertain)));
    assert!(!state.may_prompt());
    assert_eq!(state.begin(Operation::Prompt), Err(Problem::Uncertain));
    let recover = state.begin(Operation::Open).unwrap();
    assert!(state.opened(&recover, Ok(side)));
    assert!(state.may_prompt());
    assert!(state.begin(Operation::Prompt).is_ok());
}
