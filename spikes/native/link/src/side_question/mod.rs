//! A typed side-only port. It cannot express a prompt to the parent.
//! Node dispatch is attached after the centrally reviewed Step 0 merge.

use vessel_app_view::side_question::{Operation, Pending, Problem, Question, Target};
use vessel_proto::{EventRecord, SessionId, SessionInfo};

#[cfg(not(target_arch = "wasm32"))]
mod stream;

/// Independent side operations and subscription lifetime.
#[derive(Debug)]
pub enum Command {
    /// Checked operation; it cannot change the main session's subscription.
    Run(Ask),
    /// Follow the side log independently of the main Open command.
    Watch(Follow),
    /// Closing a panel cancels its stream, never the parent's stream.
    Close,
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Step 0 supplies the existing protocol types before dispatch is wired.
    pub(crate) async fn side_question(&mut self, command: Command) {
        match command {
            Command::Run(ask) => {
                let answer = Event::Answer(ask.refused(Problem::Unsupported));
                if self
                    .events
                    .send(crate::Event::SideQuestion(answer))
                    .await
                    .is_err()
                {
                    tracing::debug!("side answer arrived after the app closed");
                }
            }
            Command::Watch(follow) => {
                let client = self
                    .clients
                    .get(follow.target.machine.as_str())
                    .map(|connected| connected.client.clone());
                self.side_stream.watch(follow, client, self.events.clone());
            }
            Command::Close => self.side_stream.close(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use stream::Stream;

/// Work accepted by the side boundary; these are not new wire methods.
#[derive(Debug, Clone)]
pub enum Action {
    /// Existing `session.side_open`, scoped by the original parent.
    Open,
    /// Existing `session.side_reset`, pinned to the side the user saw.
    Reset(SessionId),
    /// Existing `session.side_prompt`, with validated side-only input.
    Prompt {
        /// Independent side session; never the original parent.
        side: SessionId,
        /// Validated at the person's input edge.
        question: Question,
    },
}

impl Action {
    const fn operation(&self) -> Operation {
        match self {
            Self::Open => Operation::Open,
            Self::Reset(_) => Operation::Reset,
            Self::Prompt { .. } => Operation::Prompt,
        }
    }
}

/// Checked operation identity and arguments, immutable after validation.
#[derive(Debug, Clone)]
pub struct Ask {
    pending: Pending,
    action: Action,
}

impl Ask {
    /// Refuse mismatched operation kinds and a side id equal to its parent.
    /// # Errors
    /// An invalid boundary request performs no node work.
    pub fn new(pending: Pending, action: Action) -> Result<Self, Problem> {
        let side = match &action {
            Action::Open => None,
            Action::Reset(side) | Action::Prompt { side, .. } => Some(*side),
        };
        if pending.operation != action.operation() || side == Some(pending.target.parent) {
            return Err(Problem::InvalidReply);
        }
        Ok(Self { pending, action })
    }

    /// Result correlation, independent of current UI selection.
    #[must_use]
    pub const fn pending(&self) -> &Pending {
        &self.pending
    }

    /// Validated side operation; ordinary session prompts are unrepresentable.
    #[must_use]
    pub const fn action(&self) -> &Action {
        &self.action
    }

    /// A queue refusal settles exactly the same request before any node work.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        let result = match self.action {
            Action::Open | Action::Reset(_) => ResultKind::Opened(Err(problem)),
            Action::Prompt { .. } => ResultKind::Prompted(Err(problem)),
        };
        Answer {
            pending: self.pending,
            result,
        }
    }
}

/// Results distinguish opening from delivering so the reducer cannot confuse them.
#[derive(Debug, Clone)]
pub enum ResultKind {
    /// Node-provided independent side, validated before it is displayed.
    Opened(Result<Box<SessionInfo>, Problem>),
    /// Delivery acknowledged; transcript still comes only from the side log.
    Prompted(Result<(), Problem>),
}

/// One operation result, retaining its original scope and identity.
#[derive(Debug, Clone)]
pub struct Answer {
    /// Immutable request correlation.
    pub pending: Pending,
    /// Typed result; no untrusted diagnostic text.
    pub result: ResultKind,
}

/// A subscription's identity survives independent of later panel operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Follow {
    /// Parent scope on the node book.
    pub target: Target,
    /// The only log to read; never the parent's.
    pub side: SessionId,
    /// Opening/reset ticket; changing it rejects a previous stream's records.
    pub ticket: vessel_proto::RequestId,
}

/// Events for the side seat only, never `Event::Records` for the main seat.
#[derive(Debug)]
pub enum Event {
    /// An operation finished.
    Answer(Answer),
    /// Records from the independently followed side.
    Records {
        /// Which subscription produced these records.
        follow: Follow,
        /// The node's records; never synthesized user/assistant messages.
        records: Vec<EventRecord>,
    },
    /// A stream stopped without a trustworthy final state.
    Lost {
        /// Which subscription stopped.
        follow: Follow,
        /// Safe reason, translated at the UI boundary.
        problem: Problem,
    },
}

#[cfg(test)]
mod tests;
