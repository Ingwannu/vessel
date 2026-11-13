use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;
use vessel_proto::{
    AgentEvent, ContentBlock, EventSeq, PermissionId, PermissionRequest, Question, QuestionId,
    SessionEvent, SessionId, SessionStatus, StopReason, ToolCall, ToolCallId, ToolCallStatus,
    ToolKind,
};

use super::*;

fn agent(event: AgentEvent) -> SessionEvent {
    SessionEvent::Agent { event }
}

fn reply(text: &str) -> SessionEvent {
    agent(AgentEvent::MessageChunk {
        content: ContentBlock::Text {
            text: text.to_owned(),
        },
    })
}

fn ended(reason: StopReason) -> SessionEvent {
    agent(AgentEvent::TurnEnded {
        reason,
        usage: None,
    })
}

#[test]
fn a_finished_turn_tells_the_end_of_its_reply_and_where_to_read() {
    let session = SessionId::random();
    let mut turn = Turn::new(session, Some("fix the build".to_owned()), EventSeq::new(7));
    assert_eq!(turn.take(&reply("x".repeat(3_000).as_str())), None);
    assert_eq!(turn.take(&reply("done.")), None);
    let notice = turn.take(&ended(StopReason::EndTurn)).unwrap();
    assert!(notice.last);
    let first =
        format!("vessel: session \"fix the build\" ({session}) finished its turn (end_turn).");
    assert!(notice.text.starts_with(&first), "{}", notice.text);
    assert!(notice.text.contains("…"), "a cut reply says so");
    assert!(notice.text.contains("done.\n\n(read_events from record 7"));
    // The reply's part only (the title has an `x` too).
    let reply = notice.text.split("\n\n").nth(1).unwrap();
    let shown = reply.chars().filter(|c| *c == 'x').count();
    assert_eq!(shown, REPLY_TAIL.saturating_sub("done.".len()));
}

#[test]
fn a_turn_that_asks_tells_each_ask_up_to_its_bound() {
    let session = SessionId::random();
    let mut turn = Turn::new(session, None, EventSeq::new(1));
    let call = ToolCall {
        id: ToolCallId::new("call-1"),
        title: "Run cargo test".to_owned(),
        kind: ToolKind::Execute,
        status: ToolCallStatus::Pending,
        content: Vec::new(),
        locations: Vec::new(),
        raw_input: None,
    };
    assert_eq!(turn.take(&agent(AgentEvent::ToolCall(call))), None);
    let asks = agent(AgentEvent::PermissionRequested(PermissionRequest {
        id: PermissionId::random(),
        tool_call: ToolCallId::new("call-1"),
        options: Vec::new(),
    }));
    let notice = turn.take(&asks).unwrap();
    assert!(!notice.last);
    assert_eq!(
        notice.text,
        format!(
            "vessel: session {session} is waiting for an answer: permission for Run cargo test."
        )
    );
    let question = agent(AgentEvent::QuestionAsked(Question {
        id: QuestionId::random(),
        message: "Which branch?".to_owned(),
        tool_call: None,
        fields: Vec::new(),
    }));
    let told = (0..20).filter_map(|_| turn.take(&question)).count();
    assert_eq!(
        told,
        usize::try_from(NEEDS_YOU_MAX.saturating_sub(1)).unwrap()
    );
}

#[test]
fn a_session_that_ends_tells_its_status() {
    let session = SessionId::random();
    let mut turn = Turn::new(session, None, EventSeq::new(1));
    let notice = turn
        .take(&SessionEvent::Status {
            status: SessionStatus::Exited { code: Some(2) },
        })
        .unwrap();
    assert!(notice.last);
    assert_eq!(
        notice.text,
        format!("vessel: session {session} ended (exited with code 2).")
    );
    let failed = turn
        .take(&agent(AgentEvent::TurnFailed {
            message: "the agent\nstopped".to_owned(),
        }))
        .unwrap();
    assert_eq!(
        failed.text,
        format!("vessel: session {session} could not finish its turn: the agent stopped.")
    );
}

#[test]
fn turns_followed_are_bounded_per_session_and_on_the_node() {
    let notices = Arc::new(Notices::default());
    let told = SessionId::random();
    let mine: Vec<_> = (0..PER_SESSION)
        .map(|_| notices.follow(told).unwrap())
        .collect();
    assert!(matches!(
        notices.follow(told),
        Err(AutomationError::Conflict(_))
    ));
    drop(mine);
    let others: Vec<_> = (0..ON_NODE)
        .map(|_| notices.follow(SessionId::random()).unwrap())
        .collect();
    assert!(matches!(
        notices.follow(told),
        Err(AutomationError::Conflict(_))
    ));
    drop(others);
    assert!(notices.follow(told).is_ok());
}

#[test]
fn at_most_sixty_notices_an_hour_go_into_one_session() {
    let notices = Notices::default();
    let told = SessionId::random();
    let start = Instant::now();
    let sent = (0..100).filter(|_| notices.may_send(told, start)).count();
    assert_eq!(sent, HOURLY);
    assert!(notices.may_send(SessionId::random(), start), "per session");
    let later = start.checked_add(Duration::from_secs(3_600)).unwrap();
    assert!(notices.may_send(told, later), "an hour on, again");
    // Sessions told nothing for an hour are not kept.
    assert_eq!(notices.state().sent.len(), 1);
}
