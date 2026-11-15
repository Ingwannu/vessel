//! One panel's target, in-flight request and safe recovery after reset.

use crate::devices::MachineKey;
use vessel_proto::automation::InvalidAutomationText;
use vessel_proto::{ErrorCode, RequestId, SessionId};

/// A side is scoped by both the node-book identity and the original session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Node-book name, parsed at that boundary.
    pub machine: MachineKey,
    /// Original structured session; never a side's id.
    pub parent: SessionId,
}

/// Which operation may settle a pending request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Find or create the current side.
    Open,
    /// Replace the exact side shown; cannot be automatically repeated.
    Reset,
    /// Deliver one validated question to the side-only ingress.
    Prompt,
}

/// Correlation travels unchanged through every node result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// Parent scope, retained even if the user navigates elsewhere.
    pub target: Target,
    /// Unique for this panel's lifetime, including close/reopen.
    pub ticket: RequestId,
    /// A reply cannot settle a different operation.
    pub operation: Operation,
}

/// Safe failures carry no question, node diagnostic or secret value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// No request can start before a parent is selected.
    #[error("no side question is open")]
    Closed,
    /// Only one side operation may be in flight.
    #[error("a side operation is already pending")]
    Busy,
    /// Request ids must never wrap and accept an old reply.
    #[error("side request identities exhausted")]
    Exhausted,
    /// `PromptText` refused the input at the edge.
    #[error("the question is empty, too long, or contains control characters")]
    InvalidQuestion(#[source] InvalidAutomationText),
    /// The node does not interpret side questions as commands.
    #[error("slash commands cannot be sent as side questions")]
    Slash,
    /// No usable side id is known yet.
    #[error("open the current side question first")]
    NoSide,
    /// The node connection is unavailable.
    #[error("the machine is offline")]
    Offline,
    /// The node pruned earlier records; retain an explicit partial-history notice.
    #[error("earlier side history was pruned")]
    HistoryGap,
    /// The negotiated node version cannot serve side questions.
    #[error("the machine does not support side questions")]
    Unsupported,
    /// Failure may have followed a mutation; never retry automatically.
    #[error("the outcome could not be confirmed")]
    Uncertain,
    /// A local bounded command queue refused before sending.
    #[error("the request was not sent")]
    Unsent,
    /// Malformed or mismatched data must not attach another conversation.
    #[error("the node returned an invalid side question")]
    InvalidReply,
    /// Node refusal, without reflecting its untrusted diagnostic text.
    #[error("the node refused the side question ({0:?})")]
    Refused(ErrorCode),
}

/// Pure state. Transcript and composer belong to the side's own UI seat.
#[derive(Debug)]
pub struct State {
    target: Option<Target>,
    side: Option<SessionId>,
    pending: Option<Pending>,
    problem: Option<Problem>,
    next: RequestId,
}

impl Default for State {
    fn default() -> Self {
        Self {
            target: None,
            side: None,
            pending: None,
            problem: None,
            next: RequestId::ZERO,
        }
    }
}

impl State {
    /// Select a parent, dropping all state from another machine/session.
    pub fn select(&mut self, target: Target) {
        if self.target.as_ref() != Some(&target) {
            self.close();
            self.target = Some(target);
        }
    }

    /// Close without recycling the source of request identities.
    pub fn close(&mut self) {
        self.target = None;
        self.side = None;
        self.pending = None;
        self.problem = None;
    }

    /// Current parent scope, used to reject records after navigation.
    #[must_use]
    pub const fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    /// The only side id that may receive input or records.
    #[must_use]
    pub const fn side(&self) -> Option<SessionId> {
        self.side
    }

    /// Work awaiting a correlated result.
    #[must_use]
    pub const fn pending(&self) -> Option<&Pending> {
        self.pending.as_ref()
    }

    /// Failure to translate beside the control, with no diagnostic echo.
    #[must_use]
    pub fn problem(&self) -> Option<Problem> {
        self.problem.clone()
    }

    /// A failed/uncertain transport must be reconciled before another question.
    /// Unsents and validation failures may be corrected without a node read.
    #[must_use]
    pub fn may_prompt(&self) -> bool {
        self.side.is_some()
            && self.pending.is_none()
            && !matches!(
                self.problem,
                Some(
                    Problem::Offline
                        | Problem::Uncertain
                        | Problem::InvalidReply
                        | Problem::Unsupported
                        | Problem::Exhausted
                )
            )
    }

    /// Begin one operation. Failed reset invalidates the old id because the
    /// node may already have deleted it; recovery is Open, never replay Reset.
    /// # Errors
    /// Refuses a closed, busy, exhausted panel or input without a side id.
    pub fn begin(&mut self, operation: Operation) -> Result<Pending, Problem> {
        let target = self.target.clone().ok_or(Problem::Closed)?;
        if self.pending.is_some() {
            return Err(Problem::Busy);
        }
        if operation != Operation::Open && self.side.is_none() {
            return Err(Problem::NoSide);
        }
        if operation == Operation::Prompt && !self.may_prompt() {
            return Err(self.problem.clone().unwrap_or(Problem::NoSide));
        }
        self.next = self.next.next().ok_or(Problem::Exhausted)?;
        let pending = Pending {
            target,
            ticket: self.next,
            operation,
        };
        self.pending = Some(pending.clone());
        self.problem = None;
        Ok(pending)
    }

    /// Settle Open/Reset only for the matching request. No late response can
    /// resurrect a closed or replaced panel, even after A-to-B-to-A navigation.
    #[must_use]
    pub fn opened(&mut self, pending: &Pending, result: Result<SessionId, Problem>) -> bool {
        if !self.accepts(pending) || pending.operation == Operation::Prompt {
            return false;
        }
        self.pending = None;
        match result {
            Ok(side) if Some(side) != self.target.as_ref().map(|target| target.parent) => {
                self.side = Some(side);
                self.problem = None;
            }
            Ok(_) => {
                self.side = None;
                self.problem = Some(Problem::InvalidReply);
            }
            Err(problem) => {
                self.side = None;
                self.problem = Some(problem);
            }
        }
        true
    }

    /// A delivered question changes no main or side timeline optimistically:
    /// only the side's own node records supply messages, exactly once.
    #[must_use]
    pub fn prompted(&mut self, pending: &Pending, result: Result<(), Problem>) -> bool {
        if !self.accepts(pending) || pending.operation != Operation::Prompt {
            return false;
        }
        self.pending = None;
        self.problem = result.err();
        true
    }

    /// Retain an input validation failure without sending or clearing text.
    pub fn refused(&mut self, problem: Problem) {
        self.problem = Some(problem);
    }

    fn accepts(&self, pending: &Pending) -> bool {
        self.target.as_ref() == Some(&pending.target) && self.pending.as_ref() == Some(pending)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
