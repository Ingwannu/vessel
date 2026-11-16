//! Shared owner task boundary for Settings and onboarding; no storage or I/O.
pub use crate::connection::ConnectionContext as Target;
use crate::devices::Ticket;
use vessel_proto::ErrorCode;
use vessel_proto::mcp::{Injection, McpSettings};
use vessel_proto::skills::{SaveSkillsSelection, SkillsSaved, SkillsSnapshot};

/// A page visit, distinct from an individual operation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit(Ticket);
impl Visit {
    /// Capture a fresh ticket from a source retained across navigation.
    #[must_use]
    pub const fn new(ticket: Ticket) -> Self {
        Self(ticket)
    }
}
/// Independent consumers cannot accept each other's equal local tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consumer {
    /// Host settings.
    Settings,
    /// Deferred onboarding batch.
    Onboarding,
}
/// The exact target and visit that owns a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Which controller owns this result.
    pub consumer: Consumer,
    /// Fixed machine and node identity.
    pub target: Target,
    /// Retained until leaving the page.
    pub visit: Visit,
}
/// Typed operations over the existing node contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Read the actual host switch and provider policy.
    ReadTools,
    /// Patch only injection, preserving unrelated provider policy.
    Inject(Injection),
    /// Read actual skill state and selection.
    ReadSkills,
    /// Install or non-destructively update selected skills.
    Reconcile,
    /// Human-confirmed whole-directory removal.
    Uninstall,
    /// Fresh-plan selection save, with exact confirmed removals.
    Save(SaveSkillsSelection),
}
impl Action {
    /// A lost acknowledgement requires a fresh read before another effect.
    #[must_use]
    pub const fn changes_node(&self) -> bool {
        match self {
            Self::ReadTools | Self::ReadSkills => false,
            Self::Inject(_) | Self::Reconcile | Self::Uninstall | Self::Save(_) => true,
        }
    }
}
/// One attempt; copied into its answer without changing the scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Owning page or wizard.
    pub scope: Scope,
    /// Unique attempt within the consumer's lifetime.
    pub ticket: Ticket,
    /// The requested effect.
    pub action: Action,
}
/// Safe typed owner results; no server error sentence crosses into UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// Actual tools setting after read or patch.
    Tools(McpSettings),
    /// Actual installed state after read/reconcile/uninstall.
    Skills(SkillsSnapshot),
    /// Save acknowledgement or fresh deletion-confirmation requirement.
    Saved(SkillsSaved),
}
/// Correlated result of a task, including queue rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// Original fixed task.
    pub task: Task,
    /// Safe data or typed failure.
    pub result: Result<Output, Problem>,
}
/// Recovery wording depends on typed facts, never private peer diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// There is no connected client.
    #[error("machine is not connected")]
    Offline,
    /// Unassigned or older protocol; do not invoke it.
    #[error("orchestration is unavailable on this machine")]
    Unsupported,
    /// A book name now points at another node or a fresh removal plan changed.
    #[error("the operation's target or confirmation changed")]
    Changed,
    /// Bounded task admission failed.
    #[error("orchestration is busy")]
    Busy,
    /// The result is unknown; the node may finish an admitted mutation.
    #[error("the operation could not be confirmed before its deadline")]
    TimedOut,
    /// An admitted effect lost its original delivery validity before installation.
    #[error("the change could not be confirmed; read its current state")]
    Uncertain,
    /// Error code only.
    #[error("the node refused the operation ({0:?})")]
    Refused(ErrorCode),
    /// Result shape is inconsistent with the requested operation.
    #[error("the operation returned an invalid result")]
    InvalidReply,
}
impl Task {
    /// A dispatch failure keeps the complete original correlation.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        Answer {
            task: self,
            result: Err(problem),
        }
    }
}
