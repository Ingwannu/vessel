//! Controller transport correlation is separate from UI epochs and child authority.
//!
//! Snapshots can outlive the actor. They retain only a receiver, so keeping a
//! snapshot cannot keep admission alive after its actual controller withdraws.

use std::sync::Arc;
use tokio::sync::watch;
use vessel_client::NodeClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Active,
    Retired,
}

/// Opaque correlation of one immutable controller transport, never a PID/epoch.
#[derive(Debug, Clone)]
pub struct TransportLease {
    identity: Arc<()>,
    state: watch::Receiver<State>,
}

impl TransportLease {
    pub(crate) fn same_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity)
    }

    pub(crate) fn active(&self) -> bool {
        *self.state.borrow() == State::Active && self.state.has_changed().is_ok()
    }

    pub(crate) async fn retired(&self) {
        let mut state = self.state.clone();
        let _retired = state.wait_for(|value| *value == State::Retired).await;
    }
}

pub(super) struct Source {
    lease: TransportLease,
    state: watch::Sender<State>,
    closed: tokio::task::JoinHandle<()>,
}

impl Source {
    pub(super) fn new(client: NodeClient) -> Self {
        let (state, receiver) = watch::channel(State::Active);
        let closed = tokio::spawn({
            let state = state.clone();
            async move {
                client.closed().await;
                let _previous = state.send_replace(State::Retired);
            }
        });
        Self {
            lease: TransportLease {
                identity: Arc::new(()),
                state: receiver,
            },
            state,
            closed,
        }
    }

    pub(super) fn lease(&self) -> TransportLease {
        self.lease.clone()
    }

    pub(super) fn retire(&self) {
        let _previous = self.state.send_replace(State::Retired);
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        self.retire();
        self.closed.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const OBSERVATION_DEADLINE: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn cloned_snapshots_cannot_keep_a_withdrawn_source_active() {
        let mut peer = crate::testing::peer::connect("transport-source").await;
        let client = peer.client.take().unwrap();
        let source = Source::new(client.clone());
        let first = source.lease();
        let duplicate = first.clone();
        let successor = Source::new(client);
        assert!(first.same_source(&duplicate));
        assert!(!first.same_source(&successor.lease()));
        drop(source);
        tokio::time::timeout(OBSERVATION_DEADLINE, duplicate.retired())
            .await
            .unwrap();
        assert!(!first.active());
        assert!(successor.lease().active());
    }

    #[tokio::test]
    async fn original_transport_closure_withdraws_its_controller_source() {
        let mut peer = crate::testing::peer::connect("closed-transport").await;
        let source = Source::new(peer.client.take().unwrap());
        let lease = source.lease();
        peer.task.take().unwrap().abort();
        tokio::time::timeout(OBSERVATION_DEADLINE, lease.retired())
            .await
            .unwrap();
        assert!(!lease.active());
    }
}
