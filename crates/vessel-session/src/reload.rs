//! Reloading a structured session's agent (`session.reload`, TRACKER 803,
//! ADR 0110): Paseo's "Reload agent". The agent is started again on the
//! same conversation with a fresh launch, so it picks up skills, MCP
//! servers and its login, and the log takes in what the agent's own
//! history has beyond it.
//!
//! Owns: the actor's reload step (`actor`), the log's user prompts
//! (`prompts`), and what the agent's replay adds (`rehydrate`).
//! Must not: unarchive, start an ended session again, mint credentials, or
//! read the store; the node does those (`vessel-node`'s `service::reload`).
//!
//! [Decision Log]
//! - Purpose: Paseo's `refresh_agent_request` (server `session.ts`): a
//!   live agent is interrupted and reloaded on its persisted session with
//!   `rehydrateFromDisk`, which reads the provider's history again.
//! - Constraint: the log is every client's cursor and is never wiped (ADR
//!   0004); a move, a wake and a start again drop the replay because the
//!   log has it (`MoveState::replaying`).
//! - Alternatives (coordinator's ruling C, 2026-11-18): drop the replay as
//!   a wake does (turns made outside Vessel never show); a reset marker
//!   with the whole replay appended again (older clients see the
//!   conversation twice, the log grows on every reload); or add only what
//!   the history has beyond the log (chosen, "R2").
//! - Choice: the replay is gathered (bounded in time and records,
//!   `rehydrate`), its user messages are lined up with the log's prompts
//!   (`prompts`), and only when the log's are exactly its first ones is the
//!   rest added after the `reloaded` record, whole messages up to a cap.
//!   Anything else adds nothing and says why in one log line.
//! - Order for a live agent, as a move's (ADR 0063's reload steps, kept):
//!   input held; a running turn cancelled and waited on for the move's
//!   grace, refused with the turn kept when it does not end (pending
//!   permissions and questions are settled as cancelled by the driver on
//!   the cancel and on the stop); the process stopped; the new one started
//!   with `session/load`. A parked agent is only started. A new agent that
//!   does not become ready, or becomes ready on another conversation, ends
//!   the session `failed` with its log kept (`session.resume` is the
//!   retry); a parked one stays parked, saying why, as a failed wake does.
//! - Cost: a reload waits for the agent's whole replay, as a wake does,
//!   and holds up to the gather's bound of it in memory until it is ready.

mod actor;
mod prompts;
mod rehydrate;
#[cfg(test)]
mod tests;

pub(crate) use actor::ReloadOrder;
pub use prompts::UserPrompts;
pub(crate) use rehydrate::Gather;

/// What the node gives a reload of a live session.
#[derive(Debug)]
pub struct Reload {
    /// What a start again would run with: the current profile resolved,
    /// the fleet's MCP servers with the agent's new `vessel` credential, a
    /// routed profile's new token, and lines for the log.
    pub launch: crate::Relaunch,
    /// The log's user prompts before this run (the store's records before
    /// [`SessionHandle::run_start`](crate::SessionHandle::run_start)).
    pub before: UserPrompts,
}

/// Why a reload was refused. Nothing was stopped, except for
/// [`TurnDidNotEnd`](Self::TurnDidNotEnd), where the turn was cancelled
/// and goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NotReloadable {
    /// An agent asked: a person reloads (Paseo has no tool for it).
    #[error("an agent may not reload a session; a person reloads it")]
    Agent,
    /// A terminal session: its harness's own interface owns its process.
    #[error("a terminal session's harness runs its own interface")]
    Terminal,
    /// Its agent never named a conversation.
    #[error("its agent has no conversation yet")]
    NoConversation,
    /// Its agent said it cannot load a conversation (no `session/load`).
    #[error("its agent cannot load a conversation (it does not offer `session/load`)")]
    AgentCannot,
    /// Its agent is starting, moving or being replaced.
    #[error("its agent is starting or moving")]
    NotReady,
    /// The cancelled turn did not end within the grace.
    #[error("the running turn did not end after its cancel; it goes on")]
    TurnDidNotEnd,
    /// The running agent could not be signalled.
    #[error("the running agent could not be stopped; nothing new was started")]
    CannotStop,
}

impl NotReloadable {
    /// Its wire code: who asked (`forbidden`), what the session is
    /// (`unsupported`), its state now (`conflict`), or its process
    /// (`unavailable`).
    pub(crate) const fn code(self) -> vessel_proto::ErrorCode {
        use vessel_proto::ErrorCode;
        match self {
            Self::Agent => ErrorCode::Forbidden,
            Self::Terminal | Self::NoConversation | Self::AgentCannot => ErrorCode::Unsupported,
            Self::NotReady | Self::TurnDidNotEnd => ErrorCode::Conflict,
            Self::CannotStop => ErrorCode::Unavailable,
        }
    }
}

impl From<crate::NotResumable> for NotReloadable {
    /// An ended session's refusal (`session.resume`'s), as a reload's.
    fn from(refusal: crate::NotResumable) -> Self {
        match refusal {
            crate::NotResumable::Terminal => Self::Terminal,
            crate::NotResumable::AgentCannot => Self::AgentCannot,
            crate::NotResumable::NoConversation => Self::NoConversation,
        }
    }
}
