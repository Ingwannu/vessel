//! Correlate one node claim with the exact transport and attention observed.
//! A lost reply spends its opportunity; it never authorizes an automatic retry.
use super::{Key, Position, Request, Source, notices::MAX_PENDING_NOTICES};

/// A node's safe delivery decision. Only Admitted allows generic OS display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// One node-owned opportunity has been spent for this connection.
    Admitted,
    /// Another client, a visible target, stale source or no present client.
    Suppressed,
}
/// Exact observation retained locally while its one claim is in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// Authenticated transport incarnation, never just its name.
    pub source: Source,
    /// Exact durable entry.
    pub key: Key,
    /// Non-reused node cursor.
    pub position: Position,
    /// Non-reused local response correlation.
    pub request: Request,
}
/// A named bound covers requests waiting for the UI as well as the RPC.
#[derive(Debug, Default)]
pub struct Gate(Vec<Pending>);
impl Gate {
    /// Duplicate or overflowing opportunities stay in the durable inbox.
    #[must_use]
    pub fn begin(&mut self, pending: Pending) -> bool {
        if self.0.len() == MAX_PENDING_NOTICES || self.0.iter().any(|item| item.key == pending.key)
        {
            return false;
        }
        self.0.push(pending);
        true
    }
    /// A stale transport or ticket can never settle a newer claim.
    #[must_use]
    pub fn take(&mut self, source: &Source, key: Key, request: Request) -> Option<Pending> {
        let index = self.0.iter().position(|item| {
            item.source == *source && item.key == key && item.request == request
        })?;
        Some(self.0.remove(index))
    }
    /// Disconnect/identity replacement invalidates queued callbacks.
    pub fn retain(&mut self, mut connected: impl FnMut(&Source) -> bool) {
        self.0.retain(|pending| connected(&pending.source));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbox::Inbox;
    #[test]
    fn another_transport_or_ticket_cannot_authorize_a_current_source() {
        let mut inbox = Inbox::default();
        let mut epochs = crate::connection::ConnectionEpochs::default();
        let context = epochs
            .admit(
                crate::connection::MachineKey::from_book("node"),
                vessel_proto::NodeId::random(),
            )
            .unwrap();
        let source = Source::new(context).unwrap();
        let key = Key {
            node: source.node(),
            session: vessel_proto::SessionId::random(),
            seq: vessel_proto::EventSeq::new(5),
        };
        let request = inbox.ticket().unwrap();
        let pending = Pending {
            source: source.clone(),
            key,
            request,
            position: Position::new(10),
        };
        let mut gate = Gate::default();
        assert!(gate.begin(pending.clone()));
        assert!(!gate.begin(pending.clone()));
        let changed = Source::new(
            epochs
                .admit(source.context().machine().clone(), source.node())
                .unwrap(),
        )
        .unwrap();
        assert!(gate.take(&changed, key, request).is_none());
        assert!(gate.take(&source, key, inbox.ticket().unwrap()).is_none());
        assert_eq!(gate.take(&source, key, request), Some(pending.clone()));
        assert!(gate.take(&source, key, request).is_none());
        assert!(gate.begin(pending));
        gate.retain(|known| *known == changed);
        assert!(gate.take(&source, key, request).is_none());
    }
}
