//! One selected authenticated admission, shared by every feature's requests.
//!
//! [Decision Log]
//! - Purpose: a saved alias can be rebound and a node can reconnect as itself.
//! - Constraint: Machine.node is the one public `NodeId` projection; Reach is
//!   a session-read result, not identity or permission.
//! - Alternatives: per-area identity maps or looking up another alias by node.
//! - Choice: the same machine snapshot carries a non-reused admitted epoch;
//!   requests capture that snapshot and the actor checks its immutable context.
//! - Why: one allocator/registry remains on the link actor, shared by features.
//! - Cost: legacy snapshots without node/epoch cannot authorize an operation.
use vessel_proto::NodeId;

/// No further safe admission identity can be allocated in this actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("connection admission identities exhausted")]
pub struct AdmissionExhausted;

/// A node-book name already parsed by the client at the book boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineKey(String);

impl MachineKey {
    /// Carry an existing node-book name into a per-machine request.
    #[must_use]
    pub fn from_book(name: &str) -> Self {
        Self(name.to_owned())
    }
    /// The name used by the connection registry.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One admitted connection in the lifetime of the app's link actor.
/// Display is for stateful control identities, never a user-facing value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConnectionEpoch(std::num::NonZeroU64);

impl std::fmt::Display for ConnectionEpoch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// Public connection facts only. A caller cannot attach scopes, credentials
/// or file paths to this identity, or substitute a display name for `NodeId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionContext {
    machine: MachineKey,
    node: NodeId,
    epoch: ConnectionEpoch,
}

impl ConnectionContext {
    /// The already-parsed book alias used only for connection lookup.
    #[must_use]
    pub const fn machine(&self) -> &MachineKey {
        &self.machine
    }

    /// The node that authenticated on this admitted connection.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }

    /// The admission, distinct even when the same node reconnects.
    #[must_use]
    pub const fn epoch(&self) -> ConnectionEpoch {
        self.epoch
    }
}

/// One actor-owned source, retained across reloads. It is deliberately not
/// Clone: copying its counter could reuse a retired admission's identity.
#[derive(Debug, Default)]
pub struct ConnectionEpochs(u64);

impl ConnectionEpochs {
    /// Reserve identity after authentication, without guessing it from a name.
    ///
    /// # Errors
    /// Exhaustion refuses admission instead of wrapping into an old context.
    pub fn admit(
        &mut self,
        machine: MachineKey,
        node: NodeId,
    ) -> Result<ConnectionContext, AdmissionExhausted> {
        // Never wrap: an old callback may still hold any earlier epoch.
        self.0 = self.0.checked_add(1).ok_or(AdmissionExhausted)?;
        Ok(ConnectionContext {
            machine,
            node,
            epoch: ConnectionEpoch(std::num::NonZeroU64::new(self.0).ok_or(AdmissionExhausted)?),
        })
    }
}

/// A selected admission is unavailable without disclosing another target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// This exact alias, node, binding or admitted generation was retired.
    #[error("the selected connection changed")]
    Changed,
    /// The original physical transport closed.
    #[error("the selected connection closed")]
    Closed,
}

/// Retirement identifies an admission without projecting `NodeId` a second time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionTag {
    machine: MachineKey,
    epoch: ConnectionEpoch,
}
impl ConnectionTag {
    /// The exact selected book entry, never an alias search by node.
    #[must_use]
    pub const fn machine(&self) -> &MachineKey {
        &self.machine
    }
    /// Only the retired generation, not a later reconnect under this alias.
    #[must_use]
    pub const fn epoch(&self) -> ConnectionEpoch {
        self.epoch
    }
}
impl ConnectionContext {
    /// Carry the same immutable admission into a retirement comparison.
    #[must_use]
    pub fn tag(&self) -> ConnectionTag {
        ConnectionTag {
            machine: self.machine.clone(),
            epoch: self.epoch,
        }
    }
    pub(crate) fn of_machine(machine: &crate::Machine) -> Option<Self> {
        Some(Self {
            machine: MachineKey::from_book(&machine.name),
            node: machine.node?,
            epoch: machine.connection_epoch?,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhaustion_never_reuses_a_generation_and_zero_is_not_a_snapshot_epoch() {
        let mut epochs = ConnectionEpochs(u64::MAX);
        for _ in 0..2 {
            assert_eq!(
                epochs.admit(MachineKey::from_book("fixture"), NodeId::random()),
                Err(AdmissionExhausted)
            );
        }
        assert!(serde_json::from_str::<ConnectionEpoch>("0").is_err());
    }
    #[test]
    fn only_the_same_machine_snapshot_can_supply_node_and_admission() {
        let mut machine = crate::Machine::unreachable("fixture".into(), "sessions refused");
        assert_eq!(machine.connection_context(), None);
        let mut epochs = ConnectionEpochs::default();
        let context = epochs
            .admit(MachineKey::from_book("fixture"), NodeId::random())
            .unwrap();
        machine.node = Some(context.node());
        assert_eq!(machine.connection_context(), None);
        machine.connection_epoch = Some(context.epoch());
        assert_eq!(machine.connection_context(), Some(context.clone()));
        let mut encoded = serde_json::to_value(&machine).unwrap();
        let _epoch = encoded.as_object_mut().unwrap().remove("connection_epoch");
        let legacy: crate::Machine = serde_json::from_value(encoded).unwrap();
        assert_eq!(legacy.node, machine.node);
        assert_eq!(legacy.connection_context(), None);
        machine.node = None;
        assert_eq!(machine.connection_context(), None);
    }
}
