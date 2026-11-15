//! Setup is fixed to the authenticated node, never just its book alias.

use super::Visit;
use crate::devices::MachineKey;
use vessel_proto::NodeId;

/// Captured by the authenticated link before setup controls become actionable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    machine: MachineKey,
    node: NodeId,
}

impl Target {
    /// Carry live authenticated connection facts across the UI boundary.
    #[must_use]
    pub const fn authenticated(machine: MachineKey, node: NodeId) -> Self {
        Self { machine, node }
    }
    /// Book key used after the link checks its current node identity.
    #[must_use]
    pub const fn machine(&self) -> &MachineKey {
        &self.machine
    }
    /// Expected identity; an alias replacement cannot change it.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
}

/// Safe refusals contain no remote text or credential-bearing address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// The chosen connection is unavailable.
    #[error("setup host is offline")]
    Offline,
    /// The alias now resolves to a different authenticated node.
    #[error("setup host identity changed")]
    Changed,
    /// The current client cannot bind native-node controls.
    #[error("setup host binding is unavailable")]
    Unavailable,
    /// Bounded admission refused before any host operation.
    #[error("setup host request was not sent")]
    Unsent,
}

/// The serialized link guard remains active until that visit is released.
#[derive(Debug, Default)]
pub struct Scope(Option<(Target, Visit)>);

impl Scope {
    /// Identity retained by the serialized link for an explicit refusal event.
    #[must_use]
    pub fn binding(&self) -> Option<(&Target, Visit)> {
        self.0.as_ref().map(|(target, visit)| (target, *visit))
    }
    /// Fence the intended target after comparing it with the live client.
    /// A failed comparison still guards the visit; it never enables controls.
    pub fn bind(&mut self, target: Target, visit: Visit) {
        self.0 = Some((target, visit));
    }
    /// An old close cannot clear a newer visit's guard.
    pub fn release(&mut self, visit: Visit) {
        if self
            .0
            .as_ref()
            .is_some_and(|(_, current)| *current == visit)
        {
            self.0 = None;
        }
    }
    /// Reused setup controls may act only on the pinned target. Ordinary
    /// session traffic uses `check()`, so unrelated conversations keep running.
    ///
    /// # Errors
    /// A settings control tried another alias or another authenticated identity.
    pub fn check_control(&self, machine: &MachineKey, node: NodeId) -> Result<(), Problem> {
        if let Some((target, _)) = &self.0
            && (target.machine() != machine || target.node() != node)
        {
            return Err(Problem::Changed);
        }
        Ok(())
    }
    /// This host must retain its identity; other hosts keep their existing policy.
    ///
    /// # Errors
    /// The protected alias was replaced by a different authenticated node.
    pub fn check(&self, machine: &MachineKey, node: NodeId) -> Result<(), Problem> {
        if let Some((target, _)) = &self.0
            && target.machine() == machine
            && target.node() != node
        {
            return Err(Problem::Changed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_replacement_and_stale_release_cannot_redirect_setup() {
        let name = MachineKey::from_book("test-node");
        let node = NodeId::random();
        let mut scope = Scope::default();
        scope.bind(Target::authenticated(name.clone(), node), Visit(2));
        scope.release(Visit(1));
        assert_eq!(scope.check(&name, NodeId::random()), Err(Problem::Changed));
        assert_eq!(scope.check(&name, node), Ok(()));
        assert_eq!(
            scope.check(&MachineKey::from_book("other-node"), NodeId::random()),
            Ok(())
        );
        assert_eq!(
            scope.check_control(&MachineKey::from_book("other-node"), NodeId::random()),
            Err(Problem::Changed)
        );
        assert_eq!(scope.check_control(&name, node), Ok(()));
        scope.release(Visit(2));
        assert_eq!(scope.check(&name, NodeId::random()), Ok(()));
    }
}
