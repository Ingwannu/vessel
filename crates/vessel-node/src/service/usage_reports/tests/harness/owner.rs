//! The outer test retains actual resources through setup/body failure and cleanup.
//!
//! [Decision Log]
//! - Purpose: a failed handshake must not strand a successfully started Node.
//! - Constraint: cancellation or dropping Node requests no completion witness.
//! - Choice: one test-local owner outside within retains Node, server and roots;
//!   the inner future unwinds/releases its physical gates before normal joins.
//! - Why: a Drop-only stop, detached reaper or replaced node would not prove the
//!   original Store/server settled. No second runner or product owner is added.
//! - Impact: original failures propagate only after actual stop/server completion;
//!   product deadlines and the existing thirty-second case remain unchanged.
use crate::limits::test_support::PhaseWait;
use futures_util::FutureExt as _;
use std::{cell::RefCell, future::Future, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Resources {
    node: Option<crate::Node>,
    server: Option<tokio::task::JoinHandle<()>>,
    stop: Option<CancellationToken>,
}

pub(in super::super) struct FixtureOwner {
    pub(super) root: Arc<tempfile::TempDir>,
    resources: RefCell<Resources>,
    started: RefCell<Vec<vessel_proto::NodeId>>,
    settled: RefCell<Vec<vessel_proto::NodeId>>,
    servers_started: std::cell::Cell<usize>,
    servers_settled: std::cell::Cell<usize>,
}
impl FixtureOwner {
    pub(in super::super) fn new() -> Self {
        Self {
            root: Arc::new(tempfile::tempdir().unwrap()),
            resources: RefCell::new(Resources::default()),
            started: RefCell::new(Vec::new()),
            settled: RefCell::new(Vec::new()),
            servers_started: std::cell::Cell::new(0),
            servers_settled: std::cell::Cell::new(0),
        }
    }
    pub(super) fn retain_node(&self, node: crate::Node) {
        self.started.borrow_mut().push(node.info().id);
        assert!(self.resources.borrow_mut().node.replace(node).is_none());
    }
    pub(super) fn retain_server(
        &self,
        server: tokio::task::JoinHandle<()>,
        stop: CancellationToken,
    ) {
        let mut resources = self.resources.borrow_mut();
        assert!(resources.server.replace(server).is_none());
        resources.stop = Some(stop);
        self.servers_started
            .set(self.servers_started.get().checked_add(1).unwrap());
    }
    pub(in super::super) fn run<'a>(
        &'a self,
        budget: Duration,
        body: impl Future<Output = ()> + 'a,
    ) -> impl Future<Output = ()> + 'a {
        // Allocate without polling, as with the phase wrapper: do not copy large
        // node setup futures through another async frame or start their clocks.
        let body = Box::pin(body);
        async move {
            // The completed/cancelled inner future is dropped at this statement,
            // releasing its SAME physical callback gates before cleanup begins.
            let result = std::panic::AssertUnwindSafe(vessel_testkit::within(budget, body))
                .catch_unwind()
                .await;
            self.finish().await;
            match result {
                Ok(result) => {
                    result.unwrap();
                }
                Err(cause) => std::panic::resume_unwind(cause),
            }
        }
    }
    async fn finish(&self) {
        let resources = self.resources.take();
        if let Some(stop) = resources.stop {
            stop.cancel();
        }
        if let Some(node) = resources.node {
            let id = node.info().id;
            node.stop()
                .phase("original usage Node/Store settlement")
                .await
                .unwrap();
            self.settled.borrow_mut().push(id);
        }
        if let Some(server) = resources.server {
            server
                .phase("original usage framed server join")
                .await
                .unwrap();
            self.servers_settled
                .set(self.servers_settled.get().checked_add(1).unwrap());
        }
    }
    pub(in super::super) fn assert_settled(&self) {
        assert_eq!(*self.started.borrow(), *self.settled.borrow());
        assert_eq!(self.started.borrow().len(), 1);
        assert_eq!(self.servers_started.get(), self.servers_settled.get());
        let resources = self.resources.borrow();
        assert!(resources.node.is_none());
        assert!(resources.server.is_none());
        assert!(
            self.root.path().is_dir(),
            "root stays owned through completion"
        );
    }
}
impl Drop for FixtureOwner {
    fn drop(&mut self) {
        // Only a last-resort backstop; run's normal stop/join is the witness.
        let resources = self.resources.get_mut();
        if let Some(stop) = &resources.stop {
            stop.cancel();
        }
        if let Some(node) = &resources.node {
            node.secure.lifecycle.stop(vessel_proto::StopWhen::Now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Fixture, SERVICE_TEST_BUDGET, SetupPhase};
    use super::FixtureOwner;
    use futures_util::FutureExt as _;
    const FAILURE_POINT: &str = "original usage fixture failure cleanup control";
    #[derive(Clone, Copy)]
    enum Failure {
        AfterNode,
        Handshake,
        Body,
    }
    #[tokio::test]
    async fn setup_handshake_and_body_failures_join_the_original_node_and_server() {
        for failure in [Failure::AfterNode, Failure::Handshake, Failure::Body] {
            let resources = FixtureOwner::new();
            let failed = std::panic::AssertUnwindSafe(resources.run(SERVICE_TEST_BUDGET, async {
                let fixture = Fixture::observed(&resources, |phase| {
                    if matches!(
                        (failure, phase),
                        (Failure::AfterNode, SetupPhase::NodeStarted)
                            | (Failure::Handshake, SetupPhase::FramedHandshake)
                    ) {
                        std::panic::panic_any(FAILURE_POINT);
                    }
                })
                .await;
                assert!(matches!(failure, Failure::Body));
                assert!(fixture.root.path().is_dir());
                std::panic::panic_any(FAILURE_POINT);
            }))
            .catch_unwind()
            .await;
            let cause = failed.expect_err("the deliberate failure must propagate after cleanup");
            assert_eq!(cause.downcast_ref::<&str>(), Some(&FAILURE_POINT));
            resources.assert_settled();
            let servers = match failure {
                Failure::AfterNode => 0,
                Failure::Handshake | Failure::Body => 1,
            };
            assert_eq!(resources.servers_settled.get(), servers);
        }
    }
}
