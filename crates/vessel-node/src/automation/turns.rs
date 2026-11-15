//! What a session's records say about its turn, for those who follow one
//! from outside: a schedule's run (`service::scheduled`) and the starter
//! told (`automation::notices`). One reading of the records, so the two
//! agree on when a turn waits, ends, or the session does.
//!
//! The matches are exhaustive on purpose (AGENTS.md section 2): an event
//! added to the protocol must be decided here.

use vessel_proto::automation::RunEnd;
use vessel_proto::{AgentEvent, ContentBlock, SessionEvent};

/// What one record means to a follower of a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Signal<'a> {
    /// A turn started.
    Started,
    /// Text of the agent's reply.
    Reply(&'a str),
    /// The turn waits for an answer: a permission, or a question (its
    /// message).
    Asks(Asking<'a>),
    /// What waited was answered.
    Answered,
    /// The turn ended, or the session did.
    Ended(RunEnd),
}

/// What a turn waits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asking<'a> {
    Permission,
    Question(&'a str),
}

/// What `event` means to a follower, if anything.
pub(crate) fn signal(event: &SessionEvent) -> Option<Signal<'_>> {
    match event {
        SessionEvent::Status { status } => (!status.is_live()).then(|| {
            Signal::Ended(RunEnd::Session {
                status: status.clone(),
            })
        }),
        SessionEvent::Agent { event } => agent(event),
        SessionEvent::Terminal(_)
        | SessionEvent::ProfileChanged { .. }
        | SessionEvent::ModelCall(_)
        | SessionEvent::OptionsChanged { .. }
        | SessionEvent::Renamed { .. }
        | SessionEvent::QueueChanged { .. }
        | SessionEvent::CommandsChanged { .. }
        | SessionEvent::UsageChanged { .. }
        | SessionEvent::Parked { .. }
        | SessionEvent::Woken
        | SessionEvent::Forked { .. }
        | SessionEvent::Rewound { .. } => None,
    }
}

fn agent(event: &AgentEvent) -> Option<Signal<'_>> {
    match event {
        AgentEvent::TurnStarted { .. } => Some(Signal::Started),
        AgentEvent::MessageChunk {
            content: ContentBlock::Text { text },
        } => Some(Signal::Reply(text)),
        AgentEvent::PermissionRequested(_) => Some(Signal::Asks(Asking::Permission)),
        AgentEvent::QuestionAsked(question) => {
            Some(Signal::Asks(Asking::Question(&question.message)))
        }
        AgentEvent::PermissionResolved { .. } | AgentEvent::QuestionResolved { .. } => {
            Some(Signal::Answered)
        }
        AgentEvent::TurnEnded { reason, .. } => {
            Some(Signal::Ended(RunEnd::Turn { reason: *reason }))
        }
        AgentEvent::TurnFailed { message } => Some(Signal::Ended(RunEnd::Failed {
            message: cut(message, vessel_proto::automation::RunOutcome::MESSAGE_MAX).to_owned(),
        })),
        AgentEvent::MessageChunk { .. }
        | AgentEvent::Ready { .. }
        | AgentEvent::UserMessageChunk { .. }
        | AgentEvent::ThoughtChunk { .. }
        | AgentEvent::ToolCall(_)
        | AgentEvent::ToolCallUpdate(_)
        | AgentEvent::Plan { .. }
        | AgentEvent::Notification(_)
        | AgentEvent::Compaction(_)
        | AgentEvent::Plugin(_)
        | AgentEvent::CommandAccepted { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::HarnessLog { .. }
        | AgentEvent::Steered { .. } => None,
    }
}

/// `text` cut at `max` bytes, on a character boundary.
pub(crate) fn cut(text: &str, max: usize) -> &str {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.get(..end).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use vessel_proto::{SessionStatus, StopReason};

    use super::*;

    #[test]
    fn a_turn_and_a_session_end_are_told_apart() {
        let ended = SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
                point: None,
            },
        };
        assert_eq!(
            signal(&ended),
            Some(Signal::Ended(RunEnd::Turn {
                reason: StopReason::EndTurn
            }))
        );
        let gone = SessionEvent::Status {
            status: SessionStatus::Exited { code: Some(0) },
        };
        assert!(matches!(
            signal(&gone),
            Some(Signal::Ended(RunEnd::Session { .. }))
        ));
        let running = SessionEvent::Status {
            status: SessionStatus::Running,
        };
        assert_eq!(signal(&running), None);
    }

    #[test]
    fn cutting_keeps_whole_characters() {
        assert_eq!(cut("héllo", 2), "h");
        assert_eq!(cut("héllo", 3), "hé");
        assert_eq!(cut("abc", 10), "abc");
    }
}
