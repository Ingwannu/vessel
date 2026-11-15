//! Errors and end reasons of this crate.

use std::fmt;
use std::sync::Arc;

use vessel_proto::conversation::InvalidAnswer;
use vessel_proto::{
    AttachmentId, AttachmentKind, OptionId, PermissionId, PermissionOptionId, QuestionId,
};

use crate::handshake::ForkRefusal;
use crate::introduction::Refused;
use crate::options::OptionError;

/// The agent process could not be started.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// The program could not be spawned.
    #[error(transparent)]
    Spawn(#[from] vessel_proc::SpawnError),
    /// No driver of that protocol is built yet (ADR 0057).
    #[error("the {0} driver is not built yet")]
    NoDriver(crate::Driver),
    /// A direct driver refused the launch before its agent started (ADR
    /// 0057): nothing was spawned.
    #[error(transparent)]
    Refused(#[from] LaunchRefusal),
}

/// Why a direct driver refused to launch its agent (ADR 0057). Checked
/// before anything starts, so a session is never started without what it
/// asked for.
///
/// [Decision Log]
/// - Purpose: a driver whose agent takes a session's choices on its
///   command line (Claude Code's permission mode, its fork, its MCP file)
///   must refuse what it cannot honour before spawning, not start the
///   agent without it.
/// - Choice: one typed refusal, so the session answers each as the ACP
///   path answers the same problem: a choice not offered is
///   `invalid_request` (as [`InputError::Option`]), a fork the agent
///   cannot make is the adapters' `unsupported` (`SessionError::ForkRefused`
///   in `vessel-session`), and the node's own private file failing is a
///   failure to start. The alternative, starting the agent and failing its
///   handshake, starts a process with flags the session did not ask for.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LaunchRefusal {
    /// The transport has no proven transient system-instruction restore path.
    #[error("the {0} driver does not support a transient system-prompt override")]
    SystemPromptUnsupported(crate::Driver),
    /// A session choice this driver cannot launch with (an option or a
    /// value it does not offer).
    #[error(transparent)]
    Option(#[from] OptionError),
    /// A fork this driver cannot make (protocol 14).
    #[error("{}", fork_refusal(*.0))]
    Fork(ForkRefusal),
    /// A conversation id this agent cannot have made, which its command
    /// line would read as something else (a path, a flag).
    #[error("`{0}` is not a conversation of this agent")]
    Conversation(String),
    /// The session has MCP servers and the node gave no private directory
    /// for the file that carries them (`AgentSetup::private_dir`): the
    /// agent is never started without them.
    #[error("this agent's MCP servers need the node's private directory, and there is none")]
    NoPrivateDir,
    /// The private file carrying the MCP servers could not be made or
    /// written (a directory not owner-only, a link, I/O).
    #[error("the agent's MCP configuration could not be written")]
    Mcp(#[source] Arc<vessel_harness::HarnessError>),
}

/// A fork refusal in a sentence.
const fn fork_refusal(refusal: ForkRefusal) -> &'static str {
    match refusal {
        ForkRefusal::CannotFork => "this agent cannot fork a conversation",
        ForkRefusal::NotAtPoint => "this agent cannot fork the conversation at that point",
    }
}

/// Input the agent session cannot accept.
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    /// A side prompt's context or final provider request exceeded its cap.
    #[error("the side question and context exceed the request limit; ask a shorter question")]
    ContextLimit,
    /// A side question's context this agent's driver does not carry yet
    /// (ADR 0057): refused before anything is sent, never dropped.
    #[error("this agent's driver does not take a side question's context yet")]
    ContextNotTaken,
    /// The ACP connection has ended.
    #[error("the agent connection has ended")]
    Closed,
    /// A turn is already running; ACP allows one prompt at a time.
    #[error("a turn is already in progress")]
    TurnInProgress,
    /// An opaque content block is not valid ACP content.
    #[error("the prompt contains content the agent protocol cannot carry")]
    InvalidContent(#[source] serde_json::Error),
    /// The prompt names an attachment whose bytes the caller did not hand
    /// over with it (protocol 10).
    #[error("attachment {0} was not loaded for the prompt")]
    AttachmentNotLoaded(AttachmentId),
    /// The prompt names an attachment of a kind the agent did not say it
    /// takes (protocol 10); nothing was sent.
    #[error("this agent does not take {} attachments (attachment {id})", noun(*.kind))]
    AttachmentNotTaken {
        /// The attachment.
        id: AttachmentId,
        /// Its kind.
        kind: AttachmentKind,
    },
    /// No such permission request was ever made in this session.
    #[error("permission request {0} does not exist")]
    PermissionNotFound(PermissionId),
    /// The permission request was already settled (the first answer wins).
    #[error("permission request {0} was already settled")]
    PermissionSettled(PermissionId),
    /// The chosen option is not one the request offered.
    #[error("permission request {id} has no option `{option}`")]
    UnknownOption {
        /// The request.
        id: PermissionId,
        /// The rejected option.
        option: PermissionOptionId,
    },
    /// No such question was ever asked in this session (protocol 10).
    #[error("question {0} does not exist")]
    QuestionNotFound(QuestionId),
    /// The question was already settled: answered, declined or withdrawn
    /// (protocol 10; the first answer wins).
    #[error("question {0} was already settled")]
    QuestionSettled(QuestionId),
    /// The answer does not fit the question's form (protocol 10); nothing
    /// was sent, and the question still waits.
    #[error("the answer does not fit question {id}")]
    InvalidAnswer {
        /// The question.
        id: QuestionId,
        /// What does not fit.
        #[source]
        reason: InvalidAnswer,
    },
    /// A config option choice the agent does not offer (protocol 9);
    /// nothing was sent to it.
    #[error(transparent)]
    Option(#[from] OptionError),
    /// The agent does not steer (protocol 10): it did not say so in its
    /// handshake, or answered that it has no such method.
    #[error("this agent cannot take a message into a running turn")]
    CannotSteer,
    /// The agent refused a steer (protocol 10).
    #[error("the agent refused the message for the running turn: {message}")]
    SteerRefused {
        /// The agent's error, for people.
        message: String,
    },
    /// The agent answered a steer outside the extension's terms (protocol
    /// 10), for example by starting a turn of its own.
    #[error("steering failed: {0}")]
    Steering(String),
    /// The agent refused a config option change (protocol 9).
    #[error("the agent refused option `{id}`: {message}")]
    Refused {
        /// The option.
        id: OptionId,
        /// The agent's error, for people.
        message: String,
    },
}

/// An attachment kind in a sentence.
const fn noun(kind: AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::Image => "image",
        AttachmentKind::Audio => "audio",
        AttachmentKind::File => "file",
    }
}

/// Why the ACP connection ended. Always the last output of an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disconnect {
    /// The agent closed its output (it exited or was stopped).
    Closed,
    /// The ACP handshake failed, so the session never became ready.
    Handshake {
        /// What went wrong, for people.
        message: String,
        /// Why, when Vessel can tell (protocol 11's `agent.status`).
        refused: Option<Box<Refused>>,
        /// A fork the agent cannot make (protocol 14, ADR 0052): the
        /// caller answers `unsupported`, not a failure to start.
        fork_refused: Option<ForkRefusal>,
    },
    /// The connection failed (a protocol or transport error).
    Protocol {
        /// What went wrong, for people.
        message: String,
    },
}

impl fmt::Display for Disconnect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => f.write_str("the agent closed its connection"),
            Self::Handshake { message, .. } => write!(f, "the ACP handshake failed: {message}"),
            Self::Protocol { message } => write!(f, "the ACP connection failed: {message}"),
        }
    }
}
