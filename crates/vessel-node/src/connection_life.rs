//! Canonical node connection identity and context-lifetime cancellation.
//!
//! A resource retains Owner and a close observer, never the lifetime guard.
//! Acting-as contexts share the guard. The close observer is also a child
//! of the transport's closing token (`Outbound::closing_token`), since
//! pending RPC calls can retain a context during transport close grace.
//! This pure primitive owns no quota, presence id, transport or worker.
//!
//! [Decision Log] (wiring, 2026-11-15)
//! - Purpose: one real identity and close observer per connection for every
//!   connection-owned resource (workspace preview, W14), reached from the
//!   call's context, instead of a copied identity per feature.
//! - Before: the primitive was test-only; the node's `Connection` had no
//!   field for it, and only `Outbound::closed()` (an async wait on a clone
//!   that holds the connection's queue) told a resource the transport left.
//! - Alternatives: a watcher task per connection that cancels the observer
//!   when `Outbound::closed()` resolves (one more task per connection, and
//!   the clone it holds keeps the queue open); leaving each resource to
//!   observe both signals itself (two signals for every consumer to combine).
//! - Choice: `Connection::new` makes the life `within` the transport's own
//!   closing token (a child of it), and `acting_as` clones it. The observer
//!   fires on whichever comes first: the transport closing or the last
//!   context going away.
//! - Costs: `vessel-rpc` gains `Outbound::closing_token`. Until the first
//!   consumer calls the accessor, the non-test build expects dead code here;
//!   that consumer removes the expectation.

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

// Tests' life with no transport under it.
#[cfg(test)]
impl Default for ConnectionLife {
    fn default() -> Self {
        Self::within(CancellationToken::new())
    }
}

impl ConnectionLife {
    /// A new connection's life, over `closing`: a token of the connection's
    /// own that is cancelled when its transport is closing. The life closes
    /// that token too, once its last context is gone, so `closing` must not
    /// be shared with another connection.
    pub(crate) fn within(closing: CancellationToken) -> Self {
        Self {
            owner: Owner(Arc::new(Identity)),
            closed: closing.clone(),
            _close_on_drop: Arc::new(closing.drop_guard()),
        }
    }

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

    #[test]
    fn a_closing_transport_ends_the_life_while_a_context_is_still_held() {
        let transport = CancellationToken::new();
        let life = ConnectionLife::within(transport.child_token());
        let closed = life.closed();
        transport.cancel();
        assert!(closed.is_cancelled());
        drop(life);
        let transport = CancellationToken::new();
        drop(ConnectionLife::within(transport.child_token()));
        assert!(
            !transport.is_cancelled(),
            "a life never closes its transport"
        );
    }
}
