//! An app-local terminal opening, echoed even by a failed history read.
//! A shared allocation is identity without a wrapping counter or a wire field.

use std::sync::Arc;
use vessel_client::book::NodeName;
use vessel_proto::SessionId;

/// One presentation of one terminal. Reopening the same session is new work.
#[derive(Debug, Clone)]
pub struct TerminalOpening {
    machine: NodeName,
    session: SessionId,
    ticket: Arc<()>,
}

impl TerminalOpening {
    /// Allocate a new ownership token, rather than recycling the session id.
    #[must_use]
    pub fn new(machine: NodeName, session: SessionId) -> Self {
        Self {
            machine,
            session,
            ticket: Arc::new(()),
        }
    }

    /// The validated book identity which owns the work.
    #[must_use]
    pub const fn machine(&self) -> &NodeName {
        &self.machine
    }

    /// The node's session, independently of this presentation's identity.
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }
}

impl PartialEq for TerminalOpening {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.ticket, &other.ticket)
            && self.machine == other.machine
            && self.session == other.session
    }
}

impl Eq for TerminalOpening {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reopen_and_other_machine_never_reuse_a_ticket() {
        let session = SessionId::random();
        let a = TerminalOpening::new("laptop".parse().unwrap(), session);
        assert_eq!(a, a.clone());
        assert_ne!(a, TerminalOpening::new("laptop".parse().unwrap(), session));
        assert_ne!(a, TerminalOpening::new("other".parse().unwrap(), session));
    }
}
