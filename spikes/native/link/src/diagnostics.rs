//! Explicit, bounded diagnostics reads on existing authenticated connections.
//! Only validated allowlisted diagnostics cross this edge; no harness discovery,
//! raw error, hostname, route, environment, key or agent output is retained.

use vessel_app_view::devices::{ConnectionContext, MachineKey, Ticket};
use vessel_app_view::diagnostics::collection::{HostResult, Problem};

/// Bound shared by the dispatcher and its per-run scheduling.
pub const MAX_PARALLEL_HOSTS: usize = 4;

/// One page-issued request; refresh allocates a different ticket.
#[derive(Debug, Clone)]
pub struct Ask {
    /// Exact alias, admitted node and non-reused connection epoch.
    pub context: ConnectionContext,
    /// The active collection identity.
    pub ticket: Ticket,
}
/// A correlated result whose fields cannot carry untrusted diagnostic prose.
#[derive(Debug, Clone)]
pub struct Answer {
    /// Admission selected when this read started, never a later alias lookup.
    context: ConnectionContext,
    /// Original collection identity.
    ticket: Ticket,
    /// Safe facts or a closed reason.
    result: Result<HostResult, Problem>,
    /// Weak original admission checked after UI queueing; absent only for unsent refusal.
    guard: Option<crate::DeliveryGuard>,
}
mod receipt;
pub use receipt::Received;
/// Answers and connection lifecycle notices share one feature event seam.
#[derive(Debug)]
pub enum Message {
    /// Exactly one correlated read outcome.
    Answered(Answer),
    /// A new authenticated connection invalidates every old stay, even for the same node id.
    Connected {
        /// Existing parsed node-book entry.
        machine: MachineKey,
        /// Identity admitted by the replacement connection.
        node: vessel_proto::NodeId,
    },
}

impl Ask {
    /// Settle refusal through the same event path as a node answer.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        Answer {
            context: self.context,
            ticket: self.ticket,
            result: Err(problem),
            guard: None,
        }
    }
}
/// A command that was never sent cannot remain pending.
#[must_use]
pub const fn rejected(why: crate::Rejected) -> Problem {
    match why {
        crate::Rejected::Full => Problem::Busy,
        crate::Rejected::Closed => Problem::Offline,
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod calls;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use calls::Hub;
