//! [`SessionError`] and its mapping to the wire.

use std::fmt;

use vessel_agent::InputError;
use vessel_harness::{LinkError, SessionFilesError};

use crate::again::NotResumable;
use crate::attachments::AttachmentError;
use crate::moving::MoveError;
use crate::queue::QueueError;
use vessel_harness::HarnessError;
use vessel_proto::{ErrorCode, SessionId, SessionInput};

/// Why a session operation failed.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The spec cannot be launched (harness or adapter missing, bad working
    /// directory).
    #[error(transparent)]
    Launch(#[from] HarnessError),
    /// The program could not be started in a PTY.
    #[error("cannot start the session")]
    Spawn(#[source] vessel_pty::PtyError),
    /// A terminal session's MCP configuration file could not be written.
    #[error("cannot write the session's MCP configuration file")]
    McpFile(#[source] std::io::Error),
    /// The ACP agent could not be started.
    #[error("cannot start the agent")]
    StartAgent(#[source] vessel_agent::AgentError),
    /// Neither the spec nor the node user's home gives a working directory,
    /// and ACP requires one.
    #[error("no working directory: the spec has none and the node user has no home directory")]
    NoWorkingDirectory,
    /// The agent refused structured input (a turn already running, a
    /// permission or question already answered, an unknown option or id,
    /// an answer that does not fit its question, content ACP cannot
    /// carry).
    #[error(transparent)]
    Input(InputError),
    /// A change to the queue, or a prompt for it, was refused (protocol
    /// 10).
    #[error(transparent)]
    Queue(#[from] QueueError),
    /// A prompt was not delivered as asked (protocol 10).
    #[error(transparent)]
    Delivery(#[from] DeliveryError),
    /// A prompt's attachments were refused or could not be loaded
    /// (protocol 10).
    #[error(transparent)]
    Attachment(#[from] AttachmentError),
    /// Something only a structured session has (protocol 10's
    /// conversation, queue, questions) was asked of a terminal session.
    #[error("a terminal session has no {0}; its harness runs its own interface")]
    Terminal(&'static str),
    /// Agent options were asked of a terminal session (protocol 9): its
    /// harness runs its own TUI, and Vessel sees no options there.
    #[error("a terminal session has no agent options; its harness's own interface sets them")]
    NoOptions,
    /// An agent could not be asked what it offers (`SessionRuntime::probe`,
    /// protocol 9).
    #[error(transparent)]
    Probe(#[from] vessel_agent::ProbeError),
    /// The input does not apply to this session's mode.
    #[error("{input} cannot be sent to a {mode} session")]
    Unsupported {
        /// What was sent.
        input: InputKind,
        /// The session's mode.
        mode: ModeKind,
    },
    /// No live session with this id in the runtime.
    #[error("no live session {0}")]
    NotFound(SessionId),
    /// `LaunchOptions::id` names a session the runtime already has (the
    /// caller's bug: ids are random).
    #[error("session {0} exists already")]
    DuplicateId(SessionId),
    /// The session is moving to another profile; input waits for that.
    #[error("the session is moving to another profile; send it again once it has moved")]
    Moving,
    /// The session did not move to another profile.
    #[error(transparent)]
    Move(#[from] MoveError),
    /// The session has ended and accepts no more input.
    #[error("the session has ended")]
    Ended,
    /// The session's agent is parked (protocol 12, ADR 0038): what goes
    /// straight to the agent (an answer) has nothing to go to; a prompt or
    /// `session.wake` starts it again.
    #[error("the session's agent is parked; a prompt or a wake starts it again")]
    Parked,
    /// The session's agent cannot be parked now (protocol 12).
    #[error("the session's agent cannot be parked: {0}")]
    NotParked(#[from] ParkRefusal),
    /// Starting a parked session's agent again failed; it stays parked.
    #[error("the session's agent did not start again: {0}")]
    WakeFailed(String),
    /// The session cannot be rewound now (protocol 14, ADR 0052).
    #[error("the session cannot be rewound now: {0}")]
    NotRewound(#[from] RewindRefusal),
    /// The agent cannot fork its conversation there (protocol 14): it does
    /// not offer `session/fork`, or it did not fork at the point.
    #[error("{}", fork_refusal(*.0))]
    ForkRefused(vessel_agent::ForkRefusal),
    /// An agent's own conversations could not be listed
    /// (`SessionRuntime::history`, protocol 17).
    #[error(transparent)]
    History(#[from] vessel_agent::HistoryError),
    /// The rewound agent did not start; the session goes on with its old
    /// conversation.
    #[error("the agent did not start at the point: {0}")]
    RewindFailed(String),
    /// Another live session is on the agent's conversation this one would
    /// start on, or starting on it (protocol 17, ADR 0053): two agents on
    /// one transcript would corrupt it.
    #[error("session {0} is already on that conversation; open it, or end it first")]
    ConversationTaken(SessionId),
    /// An ended session cannot start again (protocol 10's
    /// `session.resume`).
    #[error("the session cannot start again: {0}")]
    NotResumable(#[from] NotResumable),
    /// The runtime is shutting down and starts no new sessions.
    #[error("the node is shutting down")]
    ShuttingDown,
    /// A runtime task failed unexpectedly.
    #[error("internal session runtime failure")]
    Internal(#[source] tokio::task::JoinError),
}

/// Why a session's agent was not parked (protocol 12's `session.park`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParkRefusal {
    /// A terminal session: its harness is its own interface.
    #[error("a terminal session's harness is not parked")]
    Terminal,
    /// Its agent did not say it can resume a session (`loadSession`), or it
    /// has no session id yet: its conversation would be lost.
    #[error("its agent cannot resume a conversation")]
    CannotResume,
    /// Its agent is still starting or moving.
    #[error("its agent is starting or moving")]
    NotReady,
    /// A turn runs.
    #[error("a turn is running")]
    TurnRunning,
    /// A permission or a question waits for an answer.
    #[error("the agent waits for an answer")]
    Waiting,
    /// The queue would start its next prompt.
    #[error("the queue has prompts that are not held")]
    QueueMoving,
}

/// Why a session was not rewound (protocol 14's `session.rewind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RewindRefusal {
    /// A terminal session: Vessel does not see its agent's turns.
    #[error("a terminal session is not rewound")]
    Terminal,
    /// Its agent is still starting or moving.
    #[error("its agent is starting or moving")]
    NotReady,
    /// A turn runs.
    #[error("a turn is running")]
    TurnRunning,
    /// A permission or a question waits for an answer.
    #[error("the agent waits for an answer")]
    Waiting,
    /// The queue would start its next prompt.
    #[error("the queue has prompts that are not held")]
    QueueMoving,
}

/// A fork refusal in words.
/// A fork's refusal as a person reads it (a failed fork's status, a
/// rewind's answer).
pub(crate) const fn fork_refusal(refusal: vessel_agent::ForkRefusal) -> &'static str {
    match refusal {
        vessel_agent::ForkRefusal::CannotFork => "the agent cannot fork a conversation",
        vessel_agent::ForkRefusal::NotAtPoint => "the agent did not fork at the chosen point",
    }
}

/// Why a prompt was not delivered (protocol 10's `session.prompt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeliveryError {
    /// The bounded original receipt-ticket owner refused before dispatch.
    #[error("the session's retained input tickets are full or already preparing")]
    ReceiptCapacity,
    /// A prompt needs content.
    #[error("the prompt is empty")]
    Empty,
    /// An interrupt's cancelled turn did not end in time; nothing was
    /// sent (the turn was cancelled).
    #[error("the running turn did not end after its cancel; the prompt was not sent")]
    TurnDidNotEnd,
}

/// Why [`SessionRuntime::remove`](crate::SessionRuntime::remove) refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RemoveError {
    /// The runtime has no session with this id (never created, or already
    /// removed).
    #[error("no session {0} in the runtime")]
    NotFound(SessionId),
    /// The session can still publish records; it stays. Wait for
    /// [`SessionHandle::ended`](crate::SessionHandle::ended) (after `kill`,
    /// if it should stop).
    #[error("session {0} has not ended")]
    Running(SessionId),
}

impl SessionError {
    /// An agent that did not start, as the session answers it: a fork its
    /// direct driver refused before spawning is the adapters' refusal of
    /// the same fork (`ForkRefused`, `unsupported`; ADR 0057), so a client
    /// cannot tell which driver refused.
    pub(crate) fn agent_not_started(error: vessel_agent::AgentError) -> Self {
        if let vessel_agent::AgentError::Refused(vessel_agent::LaunchRefusal::Fork(refusal)) = error
        {
            return Self::ForkRefused(refusal);
        }
        Self::StartAgent(error)
    }

    /// The wire error code for this failure.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per kind of failure, so a new kind must choose its code; the list is \
                  the point"
    )]
    pub const fn code(&self) -> ErrorCode {
        match self {
            // An added agent's program not found is its "not installed"
            // (ADR 0032).
            Self::Launch(HarnessError::NotInstalled { .. } | HarnessError::AgentMissing { .. }) => {
                ErrorCode::HarnessNotInstalled
            }
            Self::Launch(
                HarnessError::StructuredUnavailable { .. }
                | HarnessError::NoTerminal { .. }
                | HarnessError::NoDirect { .. },
            )
            | Self::StartAgent(
                vessel_agent::AgentError::NoDriver(_)
                | vessel_agent::AgentError::Refused(vessel_agent::LaunchRefusal::Fork(_) | vessel_agent::LaunchRefusal::SystemPromptUnsupported(_)),
            )
            | Self::Unsupported { .. }
            | Self::NoOptions
            | Self::Terminal(_)
            | Self::NotResumable(_)
            | Self::NotParked(ParkRefusal::Terminal | ParkRefusal::CannotResume)
            | Self::NotRewound(RewindRefusal::Terminal)
            | Self::ForkRefused(_)
            | Self::History(vessel_agent::HistoryError::CannotList)
            | Self::Attachment(AttachmentError::NoLoader)
            | Self::Input(InputError::CannotSteer | InputError::ContextNotTaken) => {
                ErrorCode::Unsupported
            }
            Self::Launch(
                HarnessError::NoSuchDirectory { .. }
                | HarnessError::RelativeDirectory { .. }
                | HarnessError::ProfileMismatch { .. }
                | HarnessError::RoutedArgument { .. }
                | HarnessError::Mcp(_),
            )
            | Self::NoWorkingDirectory
            | Self::Delivery(DeliveryError::Empty)
            | Self::Queue(QueueError::NotNow | QueueError::TooLarge)
            | Self::Input(
                InputError::ContextLimit
            | InputError::InvalidContent(_)
                | InputError::UnknownOption { .. }
                | InputError::Option(_)
                | InputError::InvalidAnswer { .. }
                | InputError::AttachmentNotTaken { .. },
            )
            | Self::History(vessel_agent::HistoryError::Refused(_))
            // A choice a direct driver cannot launch with (ADR 0057), as a
            // running session's option change it does not offer.
            | Self::StartAgent(vessel_agent::AgentError::Refused(
                vessel_agent::LaunchRefusal::Option(_)
                | vessel_agent::LaunchRefusal::Conversation(_),
            ))
            | Self::Attachment(
                AttachmentError::TooMany { .. }
                | AttachmentError::TooLarge { .. }
                | AttachmentError::Twice(_)
                | AttachmentError::NotTaken { .. }
                | AttachmentError::NotKept(_),
            ) => ErrorCode::InvalidRequest,
            Self::Spawn(_)
            | Self::StartAgent(_)
            | Self::McpFile(_)
            | Self::Probe(_)
            | Self::History(
                vessel_agent::HistoryError::Spawn(_)
                | vessel_agent::HistoryError::Failed { .. }
                | vessel_agent::HistoryError::TimedOut { .. },
            )
            | Self::WakeFailed(_)
            | Self::RewindFailed(_) => ErrorCode::SpawnFailed,
            Self::Input(InputError::PermissionNotFound(_) | InputError::QuestionNotFound(_))
            // An agent this node has not added (ADR 0032).
            | Self::Launch(HarnessError::NotDefined { .. })
            | Self::NotFound(_)
            | Self::Queue(QueueError::NoEntry) => ErrorCode::NotFound,
            Self::Ended
            // A secret an added agent names is not set on the node: the
            // node's state, which `secret.set` changes.
            | Self::Launch(HarnessError::SecretMissing { .. })
            | Self::Moving
            | Self::ConversationTaken(_)
            // Nothing waits on a parked session: an answer has no request.
            | Self::Parked
            | Self::NotParked(
                ParkRefusal::NotReady
                | ParkRefusal::TurnRunning
                | ParkRefusal::Waiting
                | ParkRefusal::QueueMoving,
            )
            | Self::NotRewound(
                RewindRefusal::NotReady
                | RewindRefusal::TurnRunning
                | RewindRefusal::Waiting
                | RewindRefusal::QueueMoving,
            )
            // The agent's refusal of an option change: the session is in a
            // state where it does not take it.
            | Self::Input(
                InputError::Closed
                | InputError::TurnInProgress
                | InputError::PermissionSettled(_)
                | InputError::QuestionSettled(_)
                | InputError::Refused { .. }
                | InputError::SteerRefused { .. },
            )
            | Self::Queue(QueueError::Full | QueueError::Stale { .. })
            // The file is not the one kept: the node's state changed.
            | Self::Attachment(AttachmentError::Changed(_)) => ErrorCode::Conflict,
            Self::ShuttingDown | Self::Delivery(DeliveryError::TurnDidNotEnd | DeliveryError::ReceiptCapacity) => {
                ErrorCode::Unavailable
            }
            Self::Move(error) => move_code(error),
            // The node resolves `spec.profile`; another profile is its bug.
            Self::Internal(_)
            | Self::DuplicateId(_)
            | Self::Attachment(AttachmentError::Failed(_))
            // The actor loads a prompt's attachments before it sends it;
            // the agent broke the steering extension's terms.
            | Self::Input(InputError::AttachmentNotLoaded(_) | InputError::Steering(_))
            | Self::Launch(
                HarnessError::McpFile(_)
                | HarnessError::ProfileNotRequested { .. }
                | HarnessError::NoMcpDirectory
                // The node pairs a routed profile with its proxy.
                | HarnessError::Unrouted { .. },
            ) => ErrorCode::Internal,
        }
    }
}

/// A failed move's code: the client's request (`unsupported`), the
/// session's state (`conflict`), or the node (`spawn_failed`, `internal`).
const fn move_code(error: &MoveError) -> ErrorCode {
    match error {
        MoveError::Terminal | MoveError::Files(SessionFilesError::Unsupported { .. }) => {
            ErrorCode::Unsupported
        }
        MoveError::NotReady
        | MoveError::TurnInProgress
        | MoveError::Stopped
        | MoveError::Files(
            SessionFilesError::NotFound { .. }
            | SessionFilesError::Ambiguous { .. }
            | SessionFilesError::InvalidId { .. }
            | SessionFilesError::InvalidDirectory { .. },
        )
        | MoveError::Link(
            LinkError::Conflict { .. } | LinkError::Refused { .. } | LinkError::SymbolicLink { .. },
        ) => ErrorCode::Conflict,
        MoveError::Restart(_) => ErrorCode::SpawnFailed,
        MoveError::Files(SessionFilesError::Io { .. })
        | MoveError::Link(LinkError::Io { .. })
        | MoveError::Task(_)
        | MoveError::Signal { .. } => ErrorCode::Internal,
    }
}

impl From<InputError> for SessionError {
    /// A closed agent connection means the session has ended, whatever the
    /// process is still doing; everything else is the agent's verdict.
    fn from(error: InputError) -> Self {
        match error {
            InputError::Closed => Self::Ended,
            refused @ (InputError::ContextLimit
            | InputError::ContextNotTaken
            | InputError::TurnInProgress
            | InputError::InvalidContent(_)
            | InputError::PermissionNotFound(_)
            | InputError::PermissionSettled(_)
            | InputError::QuestionNotFound(_)
            | InputError::QuestionSettled(_)
            | InputError::InvalidAnswer { .. }
            | InputError::UnknownOption { .. }
            | InputError::Option(_)
            | InputError::Refused { .. }
            | InputError::AttachmentNotLoaded(_)
            | InputError::AttachmentNotTaken { .. }
            | InputError::CannotSteer
            | InputError::SteerRefused { .. }
            | InputError::Steering(_)) => Self::Input(refused),
        }
    }
}

/// A session's mode, for error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeKind {
    /// Terminal mode.
    Terminal,
    /// Structured mode.
    Structured,
}

impl fmt::Display for ModeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Terminal => "terminal",
            Self::Structured => "structured",
        })
    }
}

/// A kind of [`SessionInput`], for error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputKind {
    /// Terminal bytes or a resize.
    Terminal,
    /// A prompt.
    Prompt,
    /// A turn cancellation.
    CancelTurn,
    /// A permission answer.
    AnswerPermission,
    /// An answer to a question.
    AnswerQuestion,
}

impl InputKind {
    pub(crate) const fn of(input: &SessionInput) -> Self {
        match input {
            SessionInput::Terminal { .. } => Self::Terminal,
            SessionInput::Prompt { .. } => Self::Prompt,
            SessionInput::CancelTurn { .. } => Self::CancelTurn,
            SessionInput::AnswerPermission { .. } => Self::AnswerPermission,
            SessionInput::AnswerQuestion { .. } => Self::AnswerQuestion,
        }
    }
}

impl fmt::Display for InputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Terminal => "terminal input",
            Self::Prompt => "a prompt",
            Self::CancelTurn => "a turn cancellation",
            Self::AnswerPermission => "a permission answer",
            Self::AnswerQuestion => "an answer to a question",
        })
    }
}

#[cfg(test)]
mod tests {
    use vessel_agent::{AgentError, ForkRefusal, LaunchRefusal, OptionError};
    use vessel_proto::ErrorCode;

    use super::SessionError;

    #[test]
    fn a_transient_system_prompt_refusal_keeps_the_unsupported_error_code() {
        let error = SessionError::agent_not_started(AgentError::Refused(
            LaunchRefusal::SystemPromptUnsupported(vessel_agent::Driver::Acp),
        ));
        assert_eq!(error.code(), ErrorCode::Unsupported);
    }

    /// A direct driver's refusal before its agent starts (ADR 0057) is
    /// answered as the ACP path answers the same problem.
    #[test]
    fn a_launch_refusal_is_answered_as_the_same_refusal_over_acp() {
        let refused = |refusal| SessionError::agent_not_started(AgentError::Refused(refusal));
        let fork = refused(LaunchRefusal::Fork(ForkRefusal::NotAtPoint));
        assert!(matches!(
            fork,
            SessionError::ForkRefused(ForkRefusal::NotAtPoint)
        ));
        assert_eq!(fork.code(), ErrorCode::Unsupported);
        let option = refused(LaunchRefusal::Option(OptionError::Unknown {
            id: vessel_proto::OptionId::new("mode"),
            offered: Vec::new(),
        }));
        assert_eq!(option.code(), ErrorCode::InvalidRequest);
        let conversation = refused(LaunchRefusal::Conversation("/etc/passwd".to_owned()));
        assert_eq!(conversation.code(), ErrorCode::InvalidRequest);
        assert_eq!(
            refused(LaunchRefusal::NoPrivateDir).code(),
            ErrorCode::SpawnFailed
        );
    }
}
