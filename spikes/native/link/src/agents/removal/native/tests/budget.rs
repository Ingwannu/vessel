//! Test phases are finite; virtual product time is part of the outer budget.
//!
//! These memory-transport tests build no processes. Root `vessel_testkit::within`
//! prepares fake binaries in the root workspace, so adding that dependency
//! here would perform unrelated nested builds from the native workspace.
//! Keep its timeout semantics through the existing Tokio dev dependency.
use super::REMOVE_TIMEOUT;
use std::{future::Future, time::Duration};
use tokio::task::{AbortHandle, JoinHandle};

const PHASE_BUDGET: Duration = Duration::from_secs(5);
// Includes the intentional product-time advance plus setup, observation,
// completion and the ordered post-call read. It cannot beat that advance.
const OUTER_BUDGET: Duration = REMOVE_TIMEOUT.saturating_add(PHASE_BUDGET.saturating_mul(6));
// The server deliberately waits while the product deadline advances. A
// short receive timeout there would turn the intended wait into a disconnect.
pub(super) const PEER_IDLE_BUDGET: Duration = OUTER_BUDGET;

pub(super) async fn within<T>(name: &str, budget: Duration, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(budget, future)
        .await
        .unwrap_or_else(|_| panic!("{name} exceeded {budget:?}"))
}
pub(super) async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    within(name, PHASE_BUDGET, future).await
}
pub(super) async fn safety<T>(future: impl Future<Output = T>) -> T {
    within("removal test safety net", OUTER_BUDGET, future).await
}

// JoinHandle's Drop detaches. Retain an abort handle before any setup await
// so a failed handshake, event phase or outer timeout cannot leak its task.
pub(super) struct AbortOnDrop(AbortHandle);
impl AbortOnDrop {
    pub(super) fn new<T>(task: &JoinHandle<T>) -> Self {
        Self(task.abort_handle())
    }
}
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
