//! Request-scoped answer admission. Navigation or a new request cannot reuse an
//! outstanding ticket, and the same request cannot be answered by two local seats.

use crate::devices::MachineKey;
use vessel_proto::{ErrorCode, PermissionId, QuestionId, RequestId, SessionId};

/// The app's pending responses are bounded independently of the network queue.
pub const MAX_PENDING: usize = 32;

/// Which request an answer may resolve, without erasing its id's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Request {
    /// A provider-defined permission choice.
    Permission(PermissionId),
    /// A typed question form.
    Question(QuestionId),
}

/// Drawing scope is separate from node authorization and response admission.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ResponseScope {
    /// Main conversation shows every current pending request.
    #[default]
    All,
    /// An Inbox detail shows only its exact selected request.
    One(Request),
}

/// A request identity is local to a session on one selected node-book entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Already parsed at the node book boundary.
    pub machine: MachineKey,
    /// The request's owning session.
    pub session: SessionId,
}

/// Correlation retained unchanged across queue, call, timeout and reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// Unique over the window's lifetime, including navigation and reconnect.
    pub ticket: RequestId,
    /// A reply must not attach to a differently scoped conversation.
    pub target: Target,
    /// The exact request; a same-session successor is a different request.
    pub request: Request,
}

/// Safe categories for translated UI, never untrusted provider diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// No connection existed, so no answer was queued for later delivery.
    #[error("the machine is offline")]
    Offline,
    /// A request may have taken effect. Reconcile, never resend automatically.
    #[error("the answer could not be confirmed")]
    Unconfirmed,
    /// A local or provider response is already pending for this request.
    #[error("the request is already being answered")]
    Busy,
    /// Request identities must not wrap.
    #[error("response identities exhausted")]
    Exhausted,
    /// The selected peer cannot express this answer.
    #[error("this answer is unsupported by the machine")]
    Unsupported,
    /// A transport or caller supplied a mismatched response identity.
    #[error("the response identity is invalid")]
    Invalid,
    /// The bounded local queue refused before any node work.
    #[error("the answer was not sent")]
    Unsent,
    /// The node's typed refusal, without echoing its arbitrary text.
    #[error("the node refused the response ({0:?})")]
    Refused(ErrorCode),
}

/// Admission belongs to the window, not an individual card or its render cycle.
#[derive(Debug)]
pub struct Registry {
    next: RequestId,
    pending: Vec<Pending>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            next: RequestId::ZERO,
            pending: Vec::new(),
        }
    }
}

impl Registry {
    /// Reserve once before submitting work; a refusal has no network effect.
    /// # Errors
    /// Already pending, full or exhausted request identities refuse admission.
    pub fn begin(&mut self, target: Target, request: Request) -> Result<Pending, Problem> {
        if self.pending.len() >= MAX_PENDING
            || self
                .pending
                .iter()
                .any(|known| known.target == target && known.request == request)
        {
            return Err(Problem::Busy);
        }
        self.next = self.next.next().ok_or(Problem::Exhausted)?;
        let pending = Pending {
            ticket: self.next,
            target,
            request,
        };
        self.pending.push(pending.clone());
        Ok(pending)
    }

    /// Only the complete original identity can release its pending admission.
    #[must_use]
    pub fn settle(&mut self, pending: &Pending) -> bool {
        let Some(index) = self.pending.iter().position(|known| known == pending) else {
            return false;
        };
        let _settled = self.pending.swap_remove(index);
        true
    }
}

#[cfg(test)]
mod tests;

mod failures;
pub use failures::Failures;

mod projection;
pub use projection::retain_request;
