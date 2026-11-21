//! Waiting on another node's session as a long poll (the parent module's
//! Decision Log): a new session's wait to be ready, `wait_turn`, and the
//! pause after a node answered early.

use std::time::Duration;

use rmcp::model::JsonObject;
use serde::Serialize;
use serde_json::{Value, json};
use tokio::time::Instant;
use vessel_proto::{
    AgentEvent, ContentBlock, EventPage, EventSeq, GetSession, PermissionRequest, Question,
    ReadEvents, SessionEvent, SessionId, SessionStatus, StopReason,
};

use super::{
    EARLY_PAUSE, POLL_PAGE, POLL_SECS, READY_TIMEOUT, Target, ToolError, VesselTools, WAIT_DEFAULT,
    WAIT_MAX_SECS, number, session,
};

impl VesselTools {
    /// One long poll of `session` from `from`, for at most `wait`.
    pub(super) async fn poll(
        &self,
        node: Target,
        session: SessionId,
        from: EventSeq,
        wait: Duration,
    ) -> Result<EventPage, ToolError> {
        let seconds = u32::try_from(wait.as_secs())
            .unwrap_or(POLL_SECS)
            .clamp(1, POLL_SECS);
        self.on(
            node,
            ReadEvents {
                session,
                from,
                limit: POLL_PAGE,
                wait: Some(seconds),
            },
        )
        .await
    }

    /// Wait until a new structured session can take a prompt.
    pub(super) async fn until_running(
        &self,
        node: Target,
        session: SessionId,
    ) -> Result<(), ToolError> {
        let deadline = after(READY_TIMEOUT);
        let mut next = EventSeq::ZERO;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(ToolError::Failed(
                    "the session was not ready in time".to_owned(),
                ));
            }
            let asked = Instant::now();
            let page = self.poll(node, session, next, left).await?;
            pause_if_early(&page, asked, deadline).await;
            for record in &page.records {
                if let SessionEvent::Status { status } = &record.event {
                    if *status == SessionStatus::Running {
                        return Ok(());
                    }
                    if !status.is_live() {
                        return Err(ToolError::Failed(format!(
                            "the session ended before it was ready: {}",
                            json!(status)
                        )));
                    }
                }
            }
            next = page.next;
        }
    }

    /// Read the session from `from` until a turn ends, a permission is
    /// asked for, the session ends, or the wait runs out; returns the
    /// reply text read meanwhile and where to go on from.
    pub(super) async fn wait_turn(&self, args: &JsonObject) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        // Required: from the start, the first turn ever would answer.
        let from = number(args, "from")?
            .map(EventSeq::new)
            .ok_or_else(|| ToolError::Arguments("`from` is required".to_owned()))?;
        let limit = number(args, "timeout_secs")?.map_or(WAIT_DEFAULT, |secs| {
            Duration::from_secs(secs.clamp(1, WAIT_MAX_SECS))
        });
        let deadline = after(limit);
        let mut reply = String::new();
        let mut next = from;
        let outcome = loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break TurnOutcome::Timeout;
            }
            let asked = Instant::now();
            let page = self.poll(node, session, next, left).await?;
            pause_if_early(&page, asked, deadline).await;
            if let Some(outcome) = turn_outcome(&page, &mut reply, &mut next) {
                break outcome;
            }
            next = page.next;
            // Nothing arrived in a whole wait: an ended session has nothing
            // more to say (its final status may lie before `from`).
            if page.records.is_empty()
                && !self
                    .on(node, GetSession { session })
                    .await?
                    .status
                    .is_live()
            {
                break TurnOutcome::SessionEnded { status: None };
            }
        };
        serde_json::to_value(TurnReply {
            outcome,
            reply,
            next,
        })
        .map_err(|error| ToolError::Failed(error.to_string()))
    }
}

/// `wait_turn`'s answer: how the wait ended, the reply text read, and
/// where to read on from. Rendered once, so the JSON shape is the type.
#[derive(Debug, Serialize)]
struct TurnReply {
    #[serde(flatten)]
    outcome: TurnOutcome,
    reply: String,
    next: EventSeq,
}

/// How a `wait_turn` ended.
#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum TurnOutcome {
    /// The wait ran out.
    Timeout,
    /// The turn ended.
    Ended { reason: StopReason },
    /// The turn failed.
    Failed { message: String },
    /// The harness asked for a permission.
    PermissionRequested { request: PermissionRequest },
    /// The harness asked a question.
    QuestionAsked { question: Question },
    /// The session is no longer live. `status` is present when the page
    /// carried it.
    SessionEnded {
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<SessionStatus>,
    },
}

/// After an empty page that came back before a second had passed (the node
/// had no room for the read to wait), pause until a second has passed, or
/// until `deadline`.
pub(super) async fn pause_if_early(page: &EventPage, asked: Instant, deadline: Instant) {
    if page.records.is_empty() {
        let until = asked
            .checked_add(EARLY_PAUSE)
            .unwrap_or(deadline)
            .min(deadline);
        tokio::time::sleep_until(until).await;
    }
}

/// `wait` from now (now itself if that cannot be represented).
pub(super) fn after(wait: Duration) -> Instant {
    Instant::now()
        .checked_add(wait)
        .unwrap_or_else(Instant::now)
}

/// What `page` says of a turn, if it ends one; the reply text read and
/// where the reading got to are kept in `reply` and `next`.
fn turn_outcome(page: &EventPage, reply: &mut String, next: &mut EventSeq) -> Option<TurnOutcome> {
    for record in &page.records {
        *next = record.seq.next().unwrap_or(record.seq);
        match &record.event {
            SessionEvent::Agent {
                event:
                    AgentEvent::MessageChunk {
                        content: ContentBlock::Text { text },
                    },
            } => reply.push_str(text),
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { reason, .. },
            } => {
                return Some(TurnOutcome::Ended { reason: *reason });
            }
            SessionEvent::Agent {
                event: AgentEvent::TurnFailed { message },
            } => {
                return Some(TurnOutcome::Failed {
                    message: message.clone(),
                });
            }
            SessionEvent::Agent {
                event: AgentEvent::PermissionRequested(request),
            } => {
                return Some(TurnOutcome::PermissionRequested {
                    request: request.clone(),
                });
            }
            // Protocol 10: a question waits on an answer as a permission
            // request does, so the waiting agent hears of it the same way.
            SessionEvent::Agent {
                event: AgentEvent::QuestionAsked(question),
            } => {
                return Some(TurnOutcome::QuestionAsked {
                    question: question.clone(),
                });
            }
            SessionEvent::Status { status } if !status.is_live() => {
                return Some(TurnOutcome::SessionEnded {
                    status: Some(status.clone()),
                });
            }
            SessionEvent::Agent { .. }
            | SessionEvent::Status { .. }
            | SessionEvent::Terminal(_)
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            // A parked session waits for a prompt as an idle one does.
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }
            | SessionEvent::Reloaded { .. } => {}
        }
    }
    None
}
