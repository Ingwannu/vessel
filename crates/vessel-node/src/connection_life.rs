//! Canonical node connection identity and context-lifetime cancellation.
//!
//! A resource retains Owner and a close observer, never the lifetime guard.
//! Acting-as contexts share the guard. Wiring also observes `Outbound::closed`,
//! since pending RPC calls can retain a context during transport close grace.
//! This pure primitive owns no quota, presence id, transport or worker.

use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio_util::sync::{CancellationToken, DropGuard};

/// The actual RPC contexts keep this alive until the connection has gone away.
#[derive(Debug, Clone)]
pub(crate) struct ConnectionLife {
    owner: Owner,
    closed: CancellationToken,
    _close_on_drop: Arc<DropGuard>,
}

impl Default for ConnectionLife {
    fn default() -> Self {
        let closed = CancellationToken::new();
        let owner = Owner(Arc::new(Identity));
        Self {
            owner,
            closed: closed.clone(),
            _close_on_drop: Arc::new(closed.drop_guard()),
        }
    }
}

impl ConnectionLife {
    /// Give stream calls an identity without prolonging the real connection.
    pub(crate) fn owner(&self) -> Owner {
        self.owner.clone()
    }

    /// Streams observe closure without keeping the actual context guard alive.
    pub(crate) fn closed(&self) -> CancellationToken {
        self.closed.clone()
    }
}

/// Opaque identity retained by streams and checked on every operation.
#[derive(Debug, Clone)]
pub(crate) struct Owner(Arc<Identity>);

// Keep mutable cancellation state out of the ownership key: equality and
// hashing describe identity, while connection lifetime belongs to its guard.
#[derive(Debug)]
struct Identity;

impl PartialEq for Owner {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for Owner {}
impl Hash for Owner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn streams_do_not_keep_connections_alive_and_context_clones_do() {
        let connection = ConnectionLife::default();
        let acting_as = connection.clone();
        let owner = connection.owner();
        let stream_owner = owner.clone();
        let closed = connection.closed();
        assert_eq!(owner, acting_as.owner());
        drop(connection);
        assert!(!closed.is_cancelled());
        drop(acting_as);
        assert!(closed.is_cancelled());
        assert_eq!(owner, stream_owner);
    }

    #[test]
    fn two_live_connections_are_distinct_even_though_the_marker_has_no_id_number() {
        let first = ConnectionLife::default();
        let second = ConnectionLife::default();
        let mut owners = HashSet::new();
        assert!(owners.insert(first.owner()));
        assert!(!owners.insert(first.owner()));
        assert!(owners.insert(second.owner()));
        assert_eq!(owners.len(), 2);
        drop(first);
        assert!(!second.closed().is_cancelled());
    }
}
