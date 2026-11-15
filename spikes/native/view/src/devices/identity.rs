//! The identity a device action was drawn for, independently of its alias.
//!
//! [Decision Log]
//! - Purpose: reusing a book name, or reconnecting to the same node, must
//!   not revive a prior row, callback, confirmation or response.
//! - Constraint: the transport owns admission; view owns whether old facts
//!   may still be drawn. Neither a clock nor session-read status is identity.
//! - Alternatives: compare the alias or `NodeId` alone; both admit an old A
//!   after an observed A-to-B-to-A replacement.
//! - Choice: parsed alias, authenticated `NodeId` and a non-reused admission
//!   epoch, with a typed replacement decision for clearing owned state.
//! - Why: equality checks all three facts without a toolkit or credential.
//! - Cost: this staged model is not runtime enforcement until `Page` and the
//!   existing link actor consume it in the target-binding package.

use super::{MachineKey, Problem};
use vessel_proto::NodeId;

/// One admitted connection in the lifetime of the app's link actor.
/// Display is for stateful control identities, never a user-facing value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionEpoch(u64);

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
    ) -> Result<ConnectionContext, Problem> {
        // Never wrap: an old callback may still hold any earlier epoch.
        self.0 = self.0.checked_add(1).ok_or(Problem::Exhausted)?;
        Ok(ConnectionContext {
            machine,
            node,
            epoch: ConnectionEpoch(self.0),
        })
    }
}

/// Whether data owned by the previous context must be dropped before drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityChange {
    /// The same admitted connection; keep its current data and intent.
    Retained,
    /// A different admission, alias, node, or no longer an admitted connection.
    Replaced,
}

/// A refusal without private facts or the name of another current target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the device action belongs to a different connection")]
pub struct StaleContext;

/// The selected page's admission, not another connection registry.
#[derive(Debug, Default)]
pub struct DeviceIdentity {
    current: Option<ConnectionContext>,
}

impl DeviceIdentity {
    /// Retain only an exactly equal admission; return the required cleanup
    /// decision before any old surface is drawn or callback is handled.
    #[must_use]
    pub fn observe(&mut self, next: Option<ConnectionContext>) -> IdentityChange {
        if self.current == next {
            IdentityChange::Retained
        } else {
            self.current = next;
            IdentityChange::Replaced
        }
    }

    /// The current public identity to capture with a rendered control.
    #[must_use]
    pub const fn current(&self) -> Option<&ConnectionContext> {
        self.current.as_ref()
    }

    /// Refuse a callback or answer from any other admitted context.
    ///
    /// # Errors
    /// The selected connection was replaced, retired, or is not known yet.
    pub fn check(&self, expected: &ConnectionContext) -> Result<(), StaleContext> {
        if self.current.as_ref() == Some(expected) {
            Ok(())
        } else {
            Err(StaleContext)
        }
    }
}

#[cfg(test)]
mod tests;
