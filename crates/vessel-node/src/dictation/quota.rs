//! Stream reservations outlive cancellation while real work still owns them.
//!
//! A lease owns permits only. It never owns a connection lifetime guard, and a
//! clone moved into blocking work cannot accidentally keep that connection open.

use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

/// Bound recording reservations and the independent shared secret-read gate.
pub(super) const NODE_STREAMS: usize = 8;
const CONNECTION_STREAMS: usize = 2;

#[derive(Debug, thiserror::Error)]
/// Keep which reservation refused admission without retaining caller metadata.
pub(super) enum QuotaError {
    #[error("this connection already holds {CONNECTION_STREAMS} dictation reservations")]
    Connection(#[source] TryAcquireError),
    #[error("the node already holds {NODE_STREAMS} dictation reservations")]
    Node(#[source] TryAcquireError),
}

/// One admission counter belongs to the node's dictation service.
pub(super) struct NodeBudget(Arc<Semaphore>);
#[derive(Clone)]
/// Aliases must share this counter independently of the immutable ownership key.
pub(super) struct ConnectionBudget(Arc<Semaphore>);
/// Preparation and retirement retain this independently of the connection guard.
pub(super) struct Lease {
    _connection: OwnedSemaphorePermit,
    _node: OwnedSemaphorePermit,
}
impl NodeBudget {
    /// Seed one counter from the named node reservation bound.
    pub(super) fn new() -> Self {
        Self(Arc::new(Semaphore::new(NODE_STREAMS)))
    }
    /// Failed second acquisition returns the first permit before refusing.
    pub(super) fn take(&self, connection: &ConnectionBudget) -> Result<Arc<Lease>, QuotaError> {
        let owner = Arc::clone(&connection.0)
            .try_acquire_owned()
            .map_err(QuotaError::Connection)?;
        let node = Arc::clone(&self.0)
            .try_acquire_owned()
            .map_err(QuotaError::Node)?;
        Ok(Arc::new(Lease {
            _connection: owner,
            _node: node,
        }))
    }
}
impl ConnectionBudget {
    /// Real connection setup creates this once and clones it for aliases.
    pub(super) fn new() -> Self {
        Self(Arc::new(Semaphore::new(CONNECTION_STREAMS)))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[test]
    fn both_bounds_include_retained_leases_and_failed_admission_returns_the_other_permit() {
        let node = NodeBudget::new();
        let first = ConnectionBudget::new();
        let mut leases = Vec::new();
        for _ in 0..CONNECTION_STREAMS {
            leases.push(node.take(&first).unwrap());
        }
        assert!(matches!(node.take(&first), Err(QuotaError::Connection(_))));
        while leases.len() < NODE_STREAMS {
            let connection = ConnectionBudget::new();
            for _ in 0..CONNECTION_STREAMS {
                leases.push(node.take(&connection).unwrap());
            }
        }
        let waiting = ConnectionBudget::new();
        assert!(matches!(node.take(&waiting), Err(QuotaError::Node(_))));
        assert_eq!(waiting.0.available_permits(), CONNECTION_STREAMS);
        drop(leases.pop());
        let admitted = node.take(&waiting).unwrap();
        let retained = Arc::clone(&admitted);
        drop(admitted);
        assert!(matches!(node.take(&waiting), Err(QuotaError::Node(_))));
        drop(retained);
        let _admitted = node.take(&waiting).unwrap();
    }

    /// The reader uses real stream admission, not an independently counted fake.
    pub(in crate::dictation) fn lease() -> Arc<Lease> {
        NodeBudget::new().take(&ConnectionBudget::new()).unwrap()
    }
}
