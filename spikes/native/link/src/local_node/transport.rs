//! Controller transport correlation is separate from UI epochs and child authority.
//!
//! The transport lease retains only a receiver. A snapshot's authenticated client
//! recipe cannot keep admission authorized after its actual controller withdraws.

use std::sync::Arc;
use tokio::sync::watch;
use vessel_app_view::connection::Refusal;
use vessel_client::NodeClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Active,
    Withdrawn,
    Closed,
}

/// Opaque correlation of one immutable controller transport, never a PID/epoch.
#[derive(Debug, Clone)]
pub(crate) struct TransportLease {
    identity: Arc<()>,
    state: watch::Receiver<State>,
}

impl TransportLease {
    pub(crate) fn same_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity)
    }

    pub(crate) fn active(&self) -> bool {
        self.refusal().is_none()
    }

    /// Distinguish explicit withdrawal from physical/sender closure without I/O.
    pub(crate) fn refusal(&self) -> Option<Refusal> {
        match *self.state.borrow() {
            State::Withdrawn => Some(Refusal::Changed),
            State::Closed => Some(Refusal::Closed),
            State::Active if self.state.has_changed().is_err() => Some(Refusal::Closed),
            State::Active => None,
        }
    }

    // Actor-observer promotion awaits the explicitly granted third writer slot.
    #[cfg(test)]
    pub(crate) async fn retired(&self) -> Refusal {
        let mut state = self.state.clone();
        match state.wait_for(|value| *value != State::Active).await {
            Ok(value) if *value == State::Withdrawn => Refusal::Changed,
            Ok(_) | Err(_) => Refusal::Closed,
        }
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
                let _changed = state.send_if_modified(|value| {
                    if *value == State::Active {
                        *value = State::Closed;
                        true
                    } else {
                        false
                    }
                });
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
        let _changed = self.state.send_if_modified(|value| {
            if *value == State::Active {
                *value = State::Withdrawn;
                true
            } else {
                false
            }
        });
        // After withdrawal this observer has no reason to retain another client
        // clone while a Keep veto leaves the launcher and its reaper alive.
        self.closed.abort();
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        self.retire();
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
        assert_eq!(
            tokio::time::timeout(OBSERVATION_DEADLINE, duplicate.retired())
                .await
                .unwrap(),
            Refusal::Changed
        );
        assert!(!first.active());
        assert_eq!(first.refusal(), Some(Refusal::Changed));
        assert!(successor.lease().active());
    }

    #[tokio::test]
    async fn original_transport_closure_withdraws_its_controller_source() {
        let mut peer = crate::testing::peer::connect("closed-transport").await;
        let source = Source::new(peer.client.take().unwrap());
        let lease = source.lease();
        peer.task.take().unwrap().abort();
        assert_eq!(
            tokio::time::timeout(OBSERVATION_DEADLINE, lease.retired())
                .await
                .unwrap(),
            Refusal::Closed
        );
        assert!(!lease.active());
        assert_eq!(lease.refusal(), Some(Refusal::Closed));
        drop(source);
        assert_eq!(lease.refusal(), Some(Refusal::Closed));
    }

    #[tokio::test]
    async fn lost_sender_cannot_leave_its_last_active_value_authorized() {
        let (sender, state) = watch::channel(State::Active);
        let lease = TransportLease {
            identity: Arc::new(()),
            state,
        };
        drop(sender);
        assert_eq!(lease.refusal(), Some(Refusal::Closed));
        assert_eq!(lease.retired().await, Refusal::Closed);
    }

    #[tokio::test]
    async fn a_withdrawn_issuer_does_not_retain_its_closed_observer_client() {
        let mut peer = crate::testing::peer::connect("withdrawn-observer").await;
        let source = Source::new(peer.client.take().unwrap());
        let lease = source.lease();
        source.retire();
        // Keep the source and receiver alive: only retiring its obsolete
        // observer should drop the last client and close the real transport.
        tokio::time::timeout(OBSERVATION_DEADLINE, peer.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lease.refusal(), Some(Refusal::Changed));
        drop(source);
    }
}
