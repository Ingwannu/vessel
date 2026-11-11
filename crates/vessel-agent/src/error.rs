//! Errors and end reasons of this crate.

use std::fmt;

use vessel_proto::{AttachmentId, OptionId, PermissionId, PermissionOptionId};

use crate::options::OptionError;

/// The agent process could not be started.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// The program could not be spawned.
    #[error(transparent)]
    Spawn(#[from] vessel_proc::SpawnError),
}

/// Input the agent session cannot accept.
#[derive(Debug, thiserror::Error)]
pub enum InputError {
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

/// Why the ACP connection ended. Always the last output of an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disconnect {
    /// The agent closed its output (it exited or was stopped).
    Closed,
    /// The ACP handshake failed, so the session never became ready.
    Handshake {
        /// What went wrong, for people.
        message: String,
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
            Self::Handshake { message } => write!(f, "the ACP handshake failed: {message}"),
            Self::Protocol { message } => write!(f, "the ACP connection failed: {message}"),
        }
    }
}
