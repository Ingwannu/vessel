//! Explicit, bounded diagnostics reads on existing authenticated connections.
//! Only node identity/protocol/lifecycle cross this edge; no harness discovery,
//! raw error, hostname, route, environment, key or agent output is retained.

use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::diagnostics::collection::{HostResult, Problem};

/// Bound shared by the dispatcher and its per-run scheduling.
pub const MAX_PARALLEL_HOSTS: usize = 4;

/// One page-issued request; refresh allocates a different ticket.
#[derive(Debug, Clone)]
pub struct Ask {
    /// Parsed node-book entry.
    pub machine: MachineKey,
    /// The active collection identity.
    pub ticket: Ticket,
}
/// A correlated result whose fields cannot carry untrusted diagnostic prose.
#[derive(Debug)]
pub struct Answer {
    /// Entry selected when this read started.
    pub machine: MachineKey,
    /// Original collection identity.
    pub ticket: Ticket,
    /// Safe facts or a closed reason.
    pub result: Result<HostResult, Problem>,
}
impl Ask {
    /// Settle refusal through the same event path as a node answer.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        Answer {
            machine: self.machine,
            ticket: self.ticket,
            result: Err(problem),
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
