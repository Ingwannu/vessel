//! [`AgentInput`]: the cloneable command side of an agent session.

use tokio::sync::{mpsc, oneshot};
use vessel_proto::{ContentBlock, OptionChoice, PermissionId, PermissionOutcome, SessionOption};

use crate::error::InputError;

/// Where a command's result goes.
pub(crate) type Reply = oneshot::Sender<Result<(), InputError>>;

/// Where the options go after a change.
pub(crate) type OptionsReply = oneshot::Sender<Result<Vec<SessionOption>, InputError>>;

/// Where a steer's outcome goes.
pub(crate) type SteerReply = oneshot::Sender<Result<SteerOutcome, InputError>>;

/// What the agent did with a message for the running turn (protocol 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteerOutcome {
    /// It took it into the turn (logged as `steered`).
    Injected,
    /// No turn was running by the time it arrived; nothing was sent to
    /// the model, and the caller starts a turn with it.
    PromptRequired,
}

/// A command for the driver, answered once it has been applied.
#[derive(Debug)]
pub(crate) enum Command {
    Prompt {
        content: Vec<ContentBlock>,
        reply: Reply,
    },
    CancelTurn {
        reply: Reply,
    },
    Steer {
        content: Vec<ContentBlock>,
        reply: SteerReply,
    },
    Answer {
        id: PermissionId,
        outcome: PermissionOutcome,
        reply: Reply,
    },
    TurnRunning {
        reply: oneshot::Sender<bool>,
    },
    Options {
        reply: oneshot::Sender<Vec<SessionOption>>,
    },
    SetOption {
        choice: OptionChoice,
        reply: OptionsReply,
    },
}

/// Sends prompts, cancellations and permission answers to an agent.
///
/// Cheap to clone. Each call waits until the driver has applied it, so the
/// caller learns about a conflict (a turn already running, a permission
/// already answered) directly. Commands sent while the ACP handshake is
/// still running wait for it to finish.
#[derive(Debug, Clone)]
pub struct AgentInput {
    commands: mpsc::Sender<Command>,
}

impl AgentInput {
    pub(crate) const fn new(commands: mpsc::Sender<Command>) -> Self {
        Self { commands }
    }

    /// Start a turn with this prompt. Returns once the prompt was sent;
    /// the turn's progress and end arrive as events.
    ///
    /// # Errors
    /// [`InputError::TurnInProgress`] while a turn runs,
    /// [`InputError::InvalidContent`] for opaque blocks that are not ACP
    /// content, [`InputError::Closed`] after the connection ended.
    pub async fn prompt(&self, content: Vec<ContentBlock>) -> Result<(), InputError> {
        self.call(|reply| Command::Prompt { content, reply }).await
    }

    /// Cancel the turn in progress (a no-op when there is none). Pending
    /// permission requests are withdrawn as `Cancelled`.
    ///
    /// # Errors
    /// [`InputError::Closed`] after the connection ended.
    pub async fn cancel_turn(&self) -> Result<(), InputError> {
        self.call(|reply| Command::CancelTurn { reply }).await
    }

    /// Add `content` to the running turn (protocol 10, ACP's steering
    /// extension). With no turn running it sends nothing and answers
    /// [`SteerOutcome::PromptRequired`].
    ///
    /// # Errors
    /// [`InputError::CannotSteer`] for an agent without steering,
    /// [`InputError::SteerRefused`] when the agent refuses it,
    /// [`InputError::Steering`] when it answers outside the extension,
    /// [`InputError::InvalidContent`] for content ACP cannot carry,
    /// [`InputError::Closed`] after the connection ended.
    pub async fn steer(&self, content: Vec<ContentBlock>) -> Result<SteerOutcome, InputError> {
        let (reply, result) = oneshot::channel();
        if self
            .commands
            .send(Command::Steer { content, reply })
            .await
            .is_err()
        {
            return Err(InputError::Closed);
        }
        result.await.unwrap_or(Err(InputError::Closed))
    }

    /// Answer a pending permission request.
    ///
    /// # Errors
    /// [`InputError::PermissionSettled`] when another answer won,
    /// [`InputError::PermissionNotFound`] for an unknown id,
    /// [`InputError::UnknownOption`] for an option the request did not
    /// offer, [`InputError::Closed`] after the connection ended.
    pub async fn answer(
        &self,
        id: PermissionId,
        outcome: PermissionOutcome,
    ) -> Result<(), InputError> {
        self.call(|reply| Command::Answer { id, outcome, reply })
            .await
    }

    /// Whether a turn is running, as the driver sees it once every command
    /// sent before this one has been applied (a prompt that was already on
    /// its way counts). For moving a session to another profile, which must
    /// not stop the agent mid-turn.
    ///
    /// # Errors
    /// [`InputError::Closed`] after the connection ended.
    pub async fn turn_running(&self) -> Result<bool, InputError> {
        let (reply, result) = oneshot::channel();
        if self
            .commands
            .send(Command::TurnRunning { reply })
            .await
            .is_err()
        {
            return Err(InputError::Closed);
        }
        result.await.map_err(|_closed| InputError::Closed)
    }

    /// The agent's options as it last reported them (protocol 9). Waits
    /// for the handshake, like every command.
    ///
    /// # Errors
    /// [`InputError::Closed`] after the connection ended.
    pub async fn options(&self) -> Result<Vec<SessionOption>, InputError> {
        let (reply, result) = oneshot::channel();
        if self
            .commands
            .send(Command::Options { reply })
            .await
            .is_err()
        {
            return Err(InputError::Closed);
        }
        result.await.map_err(|_closed| InputError::Closed)
    }

    /// Change one option (protocol 9), between turns or during one, as the
    /// agent allows. Returns every option once the agent has answered; a
    /// value already in effect sends nothing and returns them at once.
    ///
    /// # Errors
    /// [`InputError::Option`] for a choice the agent does not offer,
    /// [`InputError::Refused`] when the agent refuses it,
    /// [`InputError::Closed`] after the connection ended.
    pub async fn set_option(&self, choice: OptionChoice) -> Result<Vec<SessionOption>, InputError> {
        let (reply, result) = oneshot::channel();
        if self
            .commands
            .send(Command::SetOption { choice, reply })
            .await
            .is_err()
        {
            return Err(InputError::Closed);
        }
        result.await.unwrap_or(Err(InputError::Closed))
    }

    async fn call(&self, command: impl FnOnce(Reply) -> Command) -> Result<(), InputError> {
        let (reply, result) = oneshot::channel();
        if self.commands.send(command(reply)).await.is_err() {
            return Err(InputError::Closed);
        }
        result.await.unwrap_or(Err(InputError::Closed))
    }
}
