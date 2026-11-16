//! Page-owned directory mark intent; no resident, transport or identity allocator.
use crate::{Card, Machine, connection::ConnectionContext};
use vessel_proto::{EventSeq, Mark, SessionId};

/// Immutable facts from the original rendered machine/card, not permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedSessionTarget {
    context: ConnectionContext,
    session: SessionId,
    read_through: Option<EventSeq>,
}
/// A row cannot safely supply all original intent facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
    /// Legacy, demo or unhydrated machine has no retained admission facts.
    #[error("the rendered connection identity is unavailable")]
    IdentityUnavailable,
    /// The card did not belong to this rendered machine snapshot.
    #[error("the session row belongs to another machine snapshot")]
    ForeignRow,
}
impl CapturedSessionTarget {
    /// Capture from the SAME rendered snapshot; never look up a newer alias.
    /// # Errors
    /// Refuses missing context or a card outside the supplied machine.
    pub fn capture(machine: &Machine, card: &Card) -> Result<Self, CaptureError> {
        let context = machine
            .connection_context()
            .ok_or(CaptureError::IdentityUnavailable)?;
        if card.machine != machine.name || !machine.sessions.iter().any(|row| row == card) {
            return Err(CaptureError::ForeignRow);
        }
        Ok(Self {
            context,
            session: card.session,
            read_through: card.detail.next_seq,
        })
    }
    /// The original immutable admission facts; the actual actor lease must join.
    #[must_use]
    pub const fn context(&self) -> &ConnectionContext {
        &self.context
    }
    /// Exact node-local session from the rendered row.
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }
}
/// Local page correlation only, never a resident or network capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket(u64);
/// An explicitly requested mark with its immutable original target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    ticket: Ticket,
    target: CapturedSessionTarget,
    mark: Mark,
}
impl Ask {
    /// Local pending UI request identity.
    #[must_use]
    pub const fn ticket(&self) -> Ticket {
        self.ticket
    }
    /// Original rendered target; a response cannot replace it.
    #[must_use]
    pub const fn target(&self) -> &CapturedSessionTarget {
        &self.target
    }
    /// Explicit node mark, including the original read boundary.
    #[must_use]
    pub const fn mark(&self) -> Mark {
        self.mark
    }
}
/// A refused local intent has not reached a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BeginError {
    /// Another page-owned mark is awaiting its correlated outcome.
    #[error("another session update is pending")]
    Busy,
    /// Request numbers never wrap into a late callback's identity.
    #[error("session update identities are exhausted")]
    Exhausted,
    /// Mark-read cannot borrow a later or missing cursor.
    #[error("the original row's read boundary is unavailable")]
    ReadBoundary,
}
/// One pending directory update per window; no per-session resident map.
#[derive(Debug, Default)]
pub struct Requests {
    issued: u64,
    pending: Option<Ask>,
}
impl Requests {
    /// Record the original intent before the app queues its command.
    /// # Errors
    /// Busy, exhausted correlation or a read boundary outside the original row.
    pub fn begin(&mut self, target: CapturedSessionTarget, mark: Mark) -> Result<Ask, BeginError> {
        if self.pending.is_some() {
            return Err(BeginError::Busy);
        }
        if let Mark::Read { through } = mark
            && target.read_through != Some(through)
        {
            return Err(BeginError::ReadBoundary);
        }
        let next = self.issued.checked_add(1).ok_or(BeginError::Exhausted)?;
        self.issued = next;
        let ask = Ask {
            ticket: Ticket(next),
            target,
            mark,
        };
        self.pending = Some(ask.clone());
        Ok(ask)
    }
    /// Keep another directory update from replacing this pending intent.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        self.pending.is_some()
    }
    /// Consume only the exact original UI request, including target and mark.
    pub fn finish(&mut self, ask: &Ask) -> bool {
        if self.pending.as_ref() != Some(ask) {
            return false;
        }
        self.pending = None;
        true
    }
}

#[cfg(test)]
mod tests;
