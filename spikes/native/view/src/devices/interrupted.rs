//! Losing admission after dispatch proves no outcome. Keep one bounded, public
//! notice separately from cleared credentials and authority. Reconnecting never
//! retries a write; another physical machine must not inherit this notice.
use super::{ConnectionContext, Inviting, Page, Revoking};

/// Which sent operation needs a manual check; no credential is retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unconfirmed {
    /// The node may have created an invitation, but none can be shown safely.
    Invitation,
    /// Access may already have been revoked.
    Revocation,
    /// Both requests were pending when their connection was lost.
    Both,
}
impl Unconfirmed {
    /// Uncertainty is a warning, not a claim that the node refused the action.
    #[must_use]
    pub const fn notice(self) -> crate::alert::Variant {
        crate::alert::Variant::Warning
    }
}
#[derive(Debug)]
pub(super) struct Interrupted {
    context: ConnectionContext,
    operation: Unconfirmed,
}
impl Page {
    /// Remember uncertainty before dropping the old connection's state.
    /// Confirmation alone has no effect and creates no uncertainty notice.
    pub fn interrupted(&mut self) {
        let Some(context) = self.context() else {
            return;
        };
        let operation = match (
            matches!(self.inviting, Inviting::Loading(_)),
            matches!(&self.revoking, Revoking::Sending { confirmation, .. }
                if confirmation.context == *context),
        ) {
            (true, true) => Unconfirmed::Both,
            (true, false) => Unconfirmed::Invitation,
            (false, true) => Unconfirmed::Revocation,
            (false, false) => return,
        };
        self.unconfirmed = Some(Interrupted {
            context: context.clone(),
            operation,
        });
    }
    /// Show only for this explicitly selected machine and welcome identity.
    /// A newer admission to the same node may help check the actual result.
    #[must_use]
    pub fn unconfirmed(&self, machine: &crate::Machine) -> Option<Unconfirmed> {
        self.unconfirmed
            .as_ref()
            .filter(|notice| {
                notice.context.machine().as_str() == machine.name
                    && Some(notice.context.node()) == machine.node
            })
            .map(|notice| notice.operation)
    }
}

#[cfg(test)]
mod tests;
