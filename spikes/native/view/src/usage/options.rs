//! Options retain existing connection/modal identities; no extra generation owns them.
use crate::{connection::ConnectionContext, devices::Ticket};

/// Domain context is separate from the shared popover's UI input lease.
#[derive(Debug, Clone)]
pub struct OptionsContext {
    machine: ConnectionContext,
    parent: Option<Ticket>,
}
impl OptionsContext {
    /// Capture the displayed host and optional enclosing usage modal.
    #[must_use]
    pub const fn new(machine: ConnectionContext, parent: Option<Ticket>) -> Self {
        Self { machine, parent }
    }
    /// The captured authenticated host, never a mutable label lookup.
    #[must_use]
    pub const fn machine(&self) -> &ConnectionContext {
        &self.machine
    }
    /// Retire only the child surface of the actual parent being dismissed.
    #[must_use]
    pub fn belongs_to(&self, parent: Ticket) -> bool {
        self.parent == Some(parent)
    }
    /// A same-node reconnect or a reopened parent revokes retained body actions.
    #[must_use]
    pub fn matches(&self, machine: Option<&ConnectionContext>, parent: Option<Ticket>) -> bool {
        machine == Some(&self.machine) && self.parent == parent
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        connection::{ConnectionEpochs, MachineKey},
        devices::Tickets,
    };
    #[test]
    fn retained_options_cannot_follow_a_reconnect_or_reopened_modal() {
        let mut epochs = ConnectionEpochs::default();
        let node = vessel_proto::NodeId::random();
        let first = epochs
            .admit(MachineKey::from_book("same alias"), node)
            .unwrap();
        let reconnected = epochs
            .admit(MachineKey::from_book("same alias"), node)
            .unwrap();
        let mut tickets = Tickets::default();
        let parent = tickets.next_ticket().unwrap();
        let reopened = tickets.next_ticket().unwrap();
        let options = OptionsContext::new(first.clone(), Some(parent));
        assert!(options.matches(Some(&first), Some(parent)));
        assert!(!options.matches(Some(&reconnected), Some(parent)));
        assert!(!options.matches(Some(&first), Some(reopened)));
        assert!(!options.matches(Some(&first), None));
        assert!(!options.matches(None, Some(parent)));
        assert!(!OptionsContext::new(first.clone(), None).matches(Some(&first), Some(parent)));
        assert!(options.belongs_to(parent));
        assert!(!options.belongs_to(reopened));
    }
}
