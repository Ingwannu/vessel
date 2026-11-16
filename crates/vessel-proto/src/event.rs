//! The per-session event log every client reads.
//!
//! [Decision Log]
//! - Purpose: clients disconnect, reconnect, and attach late. Each must be
//!   able to say "give me everything from N" and get an exact, gap-free
//!   continuation.
//! - Alternatives: Paseo's in-memory timeline with a random epoch per daemon
//!   run (every restart invalidates every client cursor and forces a full
//!   provider-history re-read); Zed's unacked-message replay buffer (per
//!   connection, unbounded).
//! - Choice: every session event gets a durable, gap-free `EventSeq` assigned
//!   by the session runtime and persisted by the node, so `(SessionId, seq)`
//!   is a cursor that survives reconnects and node restarts. Live delivery
//!   and paged reads (`session.read`) serve the same records.
//!
//! [Decision Log: events newer than the connection] (protocol 5)
//! - Purpose: protocol 5 adds `ProfileChanged`. A client that negotiated an
//!   older version has no variant for it and would fail to decode the
//!   record, ending its subscription or read.
//! - Constraint: the log is gap-free; leaving the record out would show the
//!   client a gap it cannot fill (every re-read would leave it out again).
//! - Alternatives: a catch-all `Unknown` variant (helps only clients built
//!   after it, not the protocol 4 clients that exist); refusing old clients
//!   (the range `MIN_PROTOCOL_VERSION..` promises they work).
//! - Choice: every event knows the version it appeared in
//!   ([`SessionEvent::required_protocol`]); for an older connection or pending
//!   assignment it has a stand-in that version can read ([`SessionEvent::for_protocol`]), with the same
//!   `seq`. `vessel-rpc` applies it where it encodes frames, the one place
//!   that knows the connection's version.
//! - Trade-off: an older client sees a stand-in, not the event: for
//!   `ProfileChanged`, `ModelCall` (protocol 7), `OptionsChanged` and
//!   `Renamed` (protocol 9), `QueueChanged`, `CommandsChanged`,
//!   `UsageChanged`, and the agent events `Steered`, `QuestionAsked` and
//!   `QuestionResolved` (protocol 10), a `HarnessLog` line starting with
//!   `vessel:`. A new variant must add its stand-in here, or the
//!   exhaustive matches stop the build.
//! - Protocol 10 is the first to add to what an existing event holds: a
//!   prompt (`TurnStarted`) may name an attachment, a content block a
//!   version 9 client cannot decode. So an agent event's version is read
//!   from its content too ([`AgentEvent::required_protocol`]), and its stand-in keeps
//!   the event, each attachment shown as a `ResourceLink` to
//!   `attachment:<id>`. Fields added as optional (`Ready.abilities`,
//!   `TurnEnded.usage`) need none: older clients skip what they do not
//!   know.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agent::{AgentEvent, ContentBlock};
use crate::conversation::{AgentCommand, ContextUsage, PromptQueue, QuestionOutcome, QueueHold};
use crate::id::EventSeq;
use crate::model::ModelCall;
use crate::name::ProfileName;
use crate::node::ProtocolVersion;
use crate::options::SessionOption;
use crate::session::SessionStatus;
use crate::terminal::TerminalOutput;
use crate::title::SessionTitle;

/// Something that happened in a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    /// The process changed state. A final `Status` is the last runtime-produced
    /// lifecycle record of a run; checkpoint metadata may follow an ended run.
    /// `session.resume` (protocol 10) starts another run in the same log.
    Status {
        /// New state.
        status: SessionStatus,
    },
    /// Terminal mode: output bytes.
    Terminal(TerminalOutput),
    /// Structured mode: a timeline entry.
    Agent {
        /// The entry.
        event: AgentEvent,
    },
    /// The session moved to another profile (protocol 5): the harness was
    /// restarted under it and resumed the same conversation.
    ProfileChanged {
        /// The profile before; `None` is the harness's default.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<ProfileName>,
        /// The profile now.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<ProfileName>,
    },
    /// The session's harness called a model through the node's proxy
    /// (protocol 7): one per upstream call, metadata only.
    ModelCall(ModelCall),
    /// The agent's config options are now these (protocol 9, ADR 0022):
    /// after the handshake, after a `session.set_option`, and whenever the
    /// agent reports a change itself. Always the whole list, so the latest
    /// one is the session's state.
    OptionsChanged {
        /// Every option, as the agent reports them.
        options: Vec<SessionOption>,
    },
    /// The session's title changed (protocol 9): from its first prompt, or
    /// by `session.rename`.
    Renamed {
        /// The title now.
        title: SessionTitle,
        /// Who produced it (ADR 0062). Absent decodes as unknown. The
        /// event stays protocol 9: a peer that ignores an unknown field
        /// still reads `title`. A typed older hop may omit the origin;
        /// the next reader then keeps the title with unknown origin.
        #[serde(default, skip_serializing_if = "crate::title::TitleOrigin::is_unknown")]
        origin: crate::title::TitleOrigin,
    },
    /// The session's queue changed (protocol 10, ADR 0024): a prompt
    /// queued, edited, moved, removed or started, or the queue held or let
    /// go. Always the whole queue, so the latest one is the session's.
    QueueChanged {
        /// The queue now.
        queue: PromptQueue,
    },
    /// The commands the agent offers (protocol 10), each time it sends
    /// them; the whole list.
    CommandsChanged {
        /// Every command, in the agent's order.
        commands: Vec<AgentCommand>,
    },
    /// The agent reported how full its context is (protocol 10).
    UsageChanged {
        /// The report.
        usage: ContextUsage,
    },
    /// The session's agent process was stopped, the session kept
    /// (protocol 12, ADR 0038); also when waking it failed.
    Parked {
        /// Why.
        reason: crate::parking::ParkReason,
    },
    /// A parked session's agent was started again with its conversation
    /// and is ready (protocol 12).
    Woken,
    /// This session is a fork of another (protocol 14, ADR 0052): its
    /// agent continues `from`'s conversation as it was at the end of the
    /// turn `after` ends (its `turn_ended` record), or at its end. The
    /// earlier conversation is in `from`'s log, not copied here.
    Forked {
        /// The session forked.
        from: crate::id::SessionId,
        /// The record of `from` the fork starts after; absent for the
        /// whole conversation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<EventSeq>,
    },
    /// The session went back to the end of the turn `after` ends (protocol
    /// 14, ADR 0052): the records between it and this one are no longer
    /// its agent's conversation. They stay in the log.
    Rewound {
        /// The `turn_ended` record it went back to.
        after: EventSeq,
        /// The agent's conversation from now on (a fork of the old one,
        /// which the agent keeps).
        harness_session: crate::id::HarnessSessionId,
        /// How many tool calls left behind edited, deleted or moved files:
        /// those changes were not put back.
        files_changed: u32,
    },
}

impl SessionEvent {
    /// The assigned protocol requirement for this event. A future variant
    /// awaiting assignment returns `None`, so projections must provide its
    /// older representation to every peer instead of inventing a number.
    #[must_use]
    pub fn required_protocol(&self) -> Option<ProtocolVersion> {
        match self {
            Self::Status { .. } | Self::Terminal(_) => Some(ProtocolVersion::new(2)),
            Self::Agent { event } => event.required_protocol(),
            Self::ProfileChanged { .. } => Some(ProtocolVersion::new(5)),
            Self::ModelCall(_) => Some(ProtocolVersion::new(7)),
            Self::OptionsChanged { .. } | Self::Renamed { .. } => Some(ProtocolVersion::new(9)),
            Self::QueueChanged { .. }
            | Self::CommandsChanged { .. }
            | Self::UsageChanged { .. } => Some(ProtocolVersion::new(10)),
            Self::Parked { .. } | Self::Woken => Some(ProtocolVersion::new(12)),
            Self::Forked { .. } | Self::Rewound { .. } => Some(ProtocolVersion::new(14)),
        }
    }

    /// What a client speaking `protocol` receives instead, or `None` when it
    /// can read this event as it is.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        if let Some(projected) = self.composer_commands_for_protocol(protocol) {
            return Some(projected);
        }
        // An assigned enclosing variant can still contain optional fields
        // introduced later. Always let the inner event mask those fields.
        if let Self::Agent { event } = self {
            return event
                .for_protocol(protocol)
                .map(|event| Self::Agent { event });
        }
        if let Some(projected) = self.composer_commands_for_protocol(protocol) {
            return Some(projected);
        }
        let readable = self
            .required_protocol()
            .is_some_and(|since| since <= protocol);
        if readable
            && let Self::UsageChanged { usage } = self
            && let Some(usage) = usage.for_protocol(protocol)
        {
            return Some(Self::UsageChanged { usage });
        }
        if readable {
            return self.origin_for_protocol(protocol, crate::GENERATED_NAMES);
        }
        let log = |line: String| {
            Some(Self::Agent {
                event: AgentEvent::HarnessLog { line },
            })
        };
        match self {
            Self::Status { .. } | Self::Terminal(_) => None,
            Self::Agent { event } => event
                .for_protocol(protocol)
                .map(|event| Self::Agent { event }),
            Self::ProfileChanged { from, to } => log(profile_line(from.as_ref(), to.as_ref())),
            Self::ModelCall(call) => log(model_line(call)),
            Self::OptionsChanged { options } => log(format!(
                "vessel: options: {}",
                SessionOption::summary(options)
            )),
            Self::Renamed { title, .. } => log(format!("vessel: renamed to {title}")),
            Self::QueueChanged { queue } => log(queue_line(queue)),
            Self::CommandsChanged { commands } => log(commands_line(commands)),
            Self::UsageChanged { usage } => log(usage_line(usage)),
            Self::Parked { reason } => log(format!(
                "vessel: the agent's process was stopped ({}); the next prompt starts it again",
                reason.describe()
            )),
            Self::Woken => log("vessel: the agent was started again".to_owned()),
            Self::Forked { from, after } => log(match after {
                Some(after) => format!(
                    "vessel: a fork of session {from} as it was after record {}",
                    after.get()
                ),
                None => format!("vessel: a fork of session {from}"),
            }),
            Self::Rewound {
                after,
                files_changed,
                ..
            } => log(format!(
                "vessel: went back to record {}; what came after is no longer the conversation{}",
                after.get(),
                if *files_changed > 0 {
                    format!(" ({files_changed} file changes were not put back)")
                } else {
                    String::new()
                }
            )),
        }
    }

    /// One origin rule shared with synthetic assigned-threshold tests,
    /// without changing the durable event's own protocol version.
    pub(crate) fn origin_for_protocol(
        &self,
        protocol: ProtocolVersion,
        assigned: Option<ProtocolVersion>,
    ) -> Option<Self> {
        if let Self::Renamed { title, origin } = self {
            let adapted = origin.for_protocol(protocol, assigned);
            return (adapted != *origin).then(|| Self::Renamed {
                title: title.clone(),
                origin: adapted,
            });
        }
        None
    }
}

impl AgentEvent {
    /// The first protocol version that can decode this event: its
    /// variant's, or 10 for a prompt holding an attachment. A feature
    /// awaiting its assigned protocol returns `None`; it must never be
    /// published as though it belonged to the current protocol.
    #[must_use]
    pub fn required_protocol(&self) -> Option<ProtocolVersion> {
        match self {
            Self::Steered { .. } | Self::QuestionAsked(_) | Self::QuestionResolved { .. } => {
                Some(ProtocolVersion::new(10))
            }
            Self::TurnStarted { content } if content.iter().any(is_attachment) => {
                Some(ProtocolVersion::new(10))
            }
            Self::UserMessageChunk { content } if is_attachment(content) => {
                Some(ProtocolVersion::new(10))
            }
            Self::Ready { .. }
            | Self::TurnStarted { .. }
            | Self::UserMessageChunk { .. }
            | Self::MessageChunk { .. }
            | Self::ThoughtChunk { .. }
            | Self::ToolCall(_)
            | Self::ToolCallUpdate(_)
            | Self::Plan { .. }
            | Self::PermissionRequested(_)
            | Self::PermissionResolved { .. }
            | Self::TurnEnded { .. }
            | Self::TurnFailed { .. }
            | Self::HarnessLog { .. } => Some(ProtocolVersion::new(2)),
        }
    }

    /// What a client speaking `protocol` receives instead, or `None` when
    /// it can read this event as it is.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        if self
            .required_protocol()
            .is_some_and(|since| since <= protocol)
        {
            return None;
        }
        let log = |line: String| Some(Self::HarnessLog { line });
        match self {
            Self::Steered { content } => log(match first_line(content) {
                Some(line) => format!("vessel: steered: {line}"),
                None => "vessel: steered".to_owned(),
            }),
            Self::QuestionAsked(question) => log(format!(
                "vessel: the agent asks: {}",
                question.message.lines().next().unwrap_or_default()
            )),
            Self::QuestionResolved { outcome, .. } => log(match outcome {
                QuestionOutcome::Answered { .. } => "vessel: question answered",
                QuestionOutcome::Declined => "vessel: question declined",
                QuestionOutcome::Cancelled => "vessel: question cancelled",
            }
            .to_owned()),
            Self::TurnStarted { content } => Some(Self::TurnStarted {
                content: content.iter().map(link_for_attachment).collect(),
            }),
            Self::UserMessageChunk { content } => Some(Self::UserMessageChunk {
                content: link_for_attachment(content),
            }),
            Self::Ready { .. }
            | Self::MessageChunk { .. }
            | Self::ThoughtChunk { .. }
            | Self::ToolCall(_)
            | Self::ToolCallUpdate(_)
            | Self::Plan { .. }
            | Self::PermissionRequested(_)
            | Self::PermissionResolved { .. }
            | Self::TurnEnded { .. }
            | Self::TurnFailed { .. }
            | Self::HarnessLog { .. } => None,
        }
    }
}

const fn is_attachment(block: &ContentBlock) -> bool {
    matches!(block, ContentBlock::Attachment { .. })
}

/// An attachment as a version 2-9 client can decode it: a link to
/// `attachment:<id>`, named after its file. Other blocks as they are.
fn link_for_attachment(block: &ContentBlock) -> ContentBlock {
    match block {
        ContentBlock::Attachment { attachment } => ContentBlock::ResourceLink {
            uri: format!("attachment:{}", attachment.id),
            name: attachment.name.to_string(),
        },
        ContentBlock::Text { .. }
        | ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. } => block.clone(),
    }
}

/// The first line of a message's first text, for a log line.
fn first_line(content: &[ContentBlock]) -> Option<&str> {
    content.iter().find_map(|block| match block {
        ContentBlock::Text { text } => text.lines().map(str::trim).find(|line| !line.is_empty()),
        ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. }
        | ContentBlock::Attachment { .. } => None,
    })
}

fn profile_line(from: Option<&ProfileName>, to: Option<&ProfileName>) -> String {
    let name = |profile: Option<&ProfileName>| {
        profile.map_or_else(|| "default".to_owned(), ToString::to_string)
    };
    format!(
        "vessel: profile changed from {} to {}",
        name(from),
        name(to)
    )
}

fn model_line(call: &ModelCall) -> String {
    match call.failure {
        None => format!(
            "vessel: model call to {}: {} in, {} out",
            call.model,
            call.usage.total_input().get(),
            call.usage.output.get()
        ),
        Some(failure) => format!(
            "vessel: model call to {} failed ({})",
            call.model,
            failure.as_str()
        ),
    }
}

fn queue_line(queue: &PromptQueue) -> String {
    let held = queue.held.map(|hold| match hold {
        QueueHold::Cancelled => "cancelled",
        QueueHold::Failed => "failed",
        QueueHold::Ended => "ended",
        QueueHold::Asked => "asked",
    });
    match held {
        Some(why) => format!(
            "vessel: queue: {} waiting (held: {why})",
            queue.entries.len()
        ),
        None => format!("vessel: queue: {} waiting", queue.entries.len()),
    }
}

/// The commands' names, the first ten of them.
fn commands_line(commands: &[AgentCommand]) -> String {
    const SHOWN: usize = 10;
    if commands.is_empty() {
        return "vessel: commands: none".to_owned();
    }
    let names: Vec<String> = commands
        .iter()
        .take(SHOWN)
        .map(|command| format!("/{}", command.name))
        .collect();
    let rest = commands.len().saturating_sub(SHOWN);
    if rest == 0 {
        format!("vessel: commands: {}", names.join(", "))
    } else {
        format!("vessel: commands: {} and {rest} more", names.join(", "))
    }
}

fn usage_line(usage: &ContextUsage) -> String {
    let context = format!(
        "vessel: context: {} of {} tokens",
        usage.used.get(),
        usage.size.get()
    );
    match &usage.cost {
        Some(cost) => format!("{context}, cost {} {}", cost.amount.get(), cost.unit),
        None => context,
    }
}

/// An event with its position and time; the unit of the session log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EventRecord {
    /// Position in the session log.
    pub seq: EventSeq,
    /// When the node recorded it.
    pub at: DateTime<Utc>,
    /// What happened.
    pub event: SessionEvent,
}

impl EventRecord {
    /// This record as a client speaking `protocol` reads it, or `None` when
    /// it needs no change. The position and time stay the same.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        self.event.for_protocol(protocol).map(|event| Self {
            seq: self.seq,
            at: self.at,
            event,
        })
    }
}

#[cfg(test)]
mod admission_tests;

#[cfg(test)]
#[path = "event/protocol_tests.rs"]
mod protocol_tests;
