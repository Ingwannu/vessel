//! A session's conversation as it stands (protocol 10's
//! `session.conversation`), kept as the session publishes.
//!
//! Owns: the projection of a structured session's log that a client asks
//! for in one call: the queue, the agent's last commands and context
//! report, its abilities, and the permission requests and questions still
//! waiting. Must not: publish anything or decide anything; the actor and
//! the agent's driver decide, this only remembers what the log says.
//!
//! [Decision Log]
//! - Purpose: a client opening a session (the CLI answering a question, a
//!   page that just connected) needs these without reading the whole log
//!   (milestone 11's rule: no whole-log read where a page would do).
//! - Choice: a projection updated by the publisher for every record it
//!   commits, as the board's activity is, so it can never disagree with
//!   the log; handles read a snapshot under a short lock. Only the records
//!   it cares about cost anything (a clone of what changed).
//! - Bounded: the waiting lists hold at most
//!   [`Seen::MAX_WAITING`] entries; the agent's driver answers anything
//!   older `cancelled`, so the oldest is the one dropped here too.

use vessel_proto::inbox::{InboxDetail, InboxItem, InboxRequest};
use vessel_proto::{
    AgentAbilities, AgentCommand, AgentEvent, AttachmentLimits, ContextUsage, Conversation,
    PermissionRequest, PromptQueue, Question, SessionEvent,
};

/// What a structured session's log says now.
#[derive(Debug, Clone, Default)]
pub(crate) struct Seen {
    queue: PromptQueue,
    commands: Vec<AgentCommand>,
    usage: Option<ContextUsage>,
    abilities: Option<AgentAbilities>,
    permissions: Vec<PermissionRequest>,
    questions: Vec<Question>,
}

impl Seen {
    /// The most waiting requests (permissions and questions each) kept:
    /// the agent driver's own bound, so both drop the same oldest one.
    pub(crate) const MAX_WAITING: usize = vessel_agent::MAX_WAITING;

    /// What a session started again (protocol 10) continues from: the
    /// queue, the last context report and the abilities its stored log
    /// left. Nothing waits, and the agent offers its commands again.
    pub(crate) const fn continuing(
        queue: PromptQueue,
        usage: Option<ContextUsage>,
        abilities: Option<AgentAbilities>,
    ) -> Self {
        Self {
            queue,
            commands: Vec::new(),
            usage,
            abilities,
            permissions: Vec::new(),
            questions: Vec::new(),
        }
    }

    /// Whether `event` changes what is kept: the publisher takes the lock
    /// for these only, so terminal output and message chunks, the bulk of
    /// a log, cost nothing here.
    pub(crate) const fn cares(event: &SessionEvent) -> bool {
        match event {
            SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Status { .. } => true,
            SessionEvent::Agent { event } => matches!(
                event,
                AgentEvent::Ready { .. }
                    | AgentEvent::PermissionRequested(_)
                    | AgentEvent::PermissionResolved { .. }
                    | AgentEvent::QuestionAsked(_)
                    | AgentEvent::QuestionResolved { .. }
                    | AgentEvent::TurnEnded { .. }
                    | AgentEvent::TurnFailed { .. }
            ),
            SessionEvent::Terminal(_)
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }
            | SessionEvent::Reloaded { .. } => false,
        }
    }

    /// Whether a permission or a question waits for an answer.
    pub(crate) const fn waiting(&self) -> bool {
        !self.permissions.is_empty() || !self.questions.is_empty()
    }

    /// Take in one committed record.
    pub(crate) fn observe(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::QueueChanged { queue } => self.queue.clone_from(queue),
            SessionEvent::CommandsChanged { commands } => self.commands.clone_from(commands),
            SessionEvent::UsageChanged { usage } => self.usage = Some(usage.clone()),
            SessionEvent::Agent { event } => self.agent(event),
            SessionEvent::Status { status } if !status.is_live() => {
                // Nothing waits on an ended session, and it offers nothing.
                self.permissions.clear();
                self.questions.clear();
                self.commands.clear();
            }
            SessionEvent::Status { .. }
            | SessionEvent::Terminal(_)
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }
            | SessionEvent::Reloaded { .. } => {}
        }
    }

    fn agent(&mut self, event: &AgentEvent) {
        match event {
            AgentEvent::Ready { abilities, .. } => {
                if abilities.is_some() {
                    self.abilities = *abilities;
                }
            }
            AgentEvent::PermissionRequested(request) => {
                push_bounded(&mut self.permissions, request.clone());
            }
            AgentEvent::PermissionResolved { id, .. } => {
                self.permissions.retain(|request| request.id != *id);
            }
            AgentEvent::QuestionAsked(question) => {
                push_bounded(&mut self.questions, question.clone());
            }
            AgentEvent::QuestionResolved { id, .. } => {
                self.questions.retain(|question| question.id != *id);
            }
            AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. } => {
                // A turn's requests end with it, answered or not.
                self.permissions.clear();
                self.questions.clear();
            }
            AgentEvent::TurnStarted { .. }
            | AgentEvent::UserMessageChunk { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::HarnessLog { .. }
            | AgentEvent::Steered { .. } => {}
        }
    }

    /// Recover exactly one actor-owned request. Cloning the whole snapshot
    /// can exceed a transport frame when several large forms are pending.
    pub(crate) fn inbox_request(&self, request: InboxRequest) -> Option<InboxItem> {
        match request {
            InboxRequest::Permission { id } => self
                .permissions
                .iter()
                .find(|request| request.id == id)
                .cloned()
                .map(|request| InboxItem::Permission {
                    request,
                    tool: None,
                    detail: InboxDetail::InSession,
                }),
            InboxRequest::Question { id } => self
                .questions
                .iter()
                .find(|question| question.id == id)
                .cloned()
                .map(|question| InboxItem::Question { question }),
        }
    }

    /// The conversation as a client reads it.
    pub(crate) fn conversation(&self) -> Conversation {
        Conversation {
            queue: self.queue.clone(),
            commands: self.commands.clone(),
            usage: self.usage.clone(),
            permissions: self.permissions.clone(),
            questions: self.questions.clone(),
            abilities: self.abilities,
            limits: AttachmentLimits::DEFAULT,
        }
    }
}

fn push_bounded<T>(waiting: &mut Vec<T>, item: T) {
    if waiting.len() >= Seen::MAX_WAITING {
        let _oldest = waiting.remove(0);
    }
    waiting.push(item);
}

#[cfg(test)]
mod tests {
    use vessel_proto::{
        HarnessSessionId, PermissionId, PermissionOutcome, SessionStatus, StopReason, ToolCallId,
    };

    use super::*;

    fn permission() -> PermissionRequest {
        PermissionRequest {
            id: PermissionId::random(),
            tool_call: ToolCallId::new("call"),
            options: Vec::new(),
        }
    }

    fn agent(event: AgentEvent) -> SessionEvent {
        SessionEvent::Agent { event }
    }

    #[test]
    fn what_waits_is_what_was_asked_and_not_answered() {
        let mut seen = Seen::default();
        let abilities = AgentAbilities {
            steer: true,
            ..AgentAbilities::default()
        };
        seen.observe(&agent(AgentEvent::Ready {
            harness_session: HarnessSessionId::new("s"),
            abilities: Some(abilities),
        }));
        let first = permission();
        let second = permission();
        seen.observe(&agent(AgentEvent::PermissionRequested(first.clone())));
        seen.observe(&agent(AgentEvent::PermissionRequested(second.clone())));
        seen.observe(&agent(AgentEvent::PermissionResolved {
            id: first.id,
            outcome: PermissionOutcome::Cancelled,
        }));
        let conversation = seen.conversation();
        assert_eq!(conversation.permissions, [second]);
        assert_eq!(conversation.abilities, Some(abilities));
        seen.observe(&agent(AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        }));
        assert!(seen.conversation().permissions.is_empty());
    }

    #[test]
    fn one_live_request_is_exact_and_disappears_after_resolution() {
        let mut seen = Seen::default();
        let first = permission();
        let second = permission();
        seen.observe(&agent(AgentEvent::PermissionRequested(first.clone())));
        seen.observe(&agent(AgentEvent::PermissionRequested(second)));
        let key = InboxRequest::Permission { id: first.id };
        assert_eq!(
            seen.inbox_request(key),
            Some(InboxItem::Permission {
                request: first.clone(),
                tool: None,
                detail: InboxDetail::InSession
            })
        );
        seen.observe(&agent(AgentEvent::PermissionResolved {
            id: first.id,
            outcome: PermissionOutcome::Cancelled,
        }));
        assert_eq!(seen.inbox_request(key), None);
    }
    #[test]
    fn waiting_requests_are_bounded_and_end_with_the_session() {
        let mut seen = Seen::default();
        for _ in 0..=Seen::MAX_WAITING {
            seen.observe(&agent(AgentEvent::PermissionRequested(permission())));
        }
        assert_eq!(seen.conversation().permissions.len(), Seen::MAX_WAITING);
        seen.observe(&SessionEvent::CommandsChanged {
            commands: vec![AgentCommand {
                name: "compact".parse().unwrap(),
                description: String::new(),
                hint: None,
                kind: None,
            }],
        });
        seen.observe(&SessionEvent::Status {
            status: SessionStatus::Exited { code: Some(0) },
        });
        let ended = seen.conversation();
        assert!(ended.permissions.is_empty());
        assert!(ended.commands.is_empty());
        assert_eq!(ended.abilities, None, "never reported");
    }
}
