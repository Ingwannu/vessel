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
//! - Cost: the actor owns one epoch source and `Page` owns only its selection;
//!   neither identity nor a successful read implies caller authority.

use crate::connection::ConnectionContext;

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
