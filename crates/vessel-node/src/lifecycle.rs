//! The node's own stop requests (protocol 3 `node.stop`) and draining.
//!
//! [Decision Log]
//! - Purpose: a remote update must be able to stop a node without ending
//!   live sessions unless the user chose that (ADR 0008). `now` is the M1
//!   shutdown; `when_idle` refuses new sessions and stops once none is
//!   live.
//! - Choice: both requests cancel the same token that `Node::stop` cancels,
//!   so every stop goes through the one shutdown order of `shutdown.rs`
//!   (which also waits for the last records to be committed). Draining is a
//!   state in the node's published `NodeInfo` (`NodeState::Draining`), read
//!   by `session.create` to refuse and by every new `Welcome`; a task waits
//!   for live sessions to end on their own watches, never by polling.
//! - Known gap: a `session.create` that passed the draining check just
//!   before draining began can still start its session; the waiting task
//!   lists sessions again after each round, so it waits for that one too,
//!   unless the node was already idle and stopping.

use std::sync::{Arc, Weak};

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use vessel_proto::{NodeInfo, NodeState, StopWhen};
use vessel_session::{SessionHandle, SessionRuntime};

/// Stop and drain control, shared by the service and the node.
#[derive(Debug, Clone)]
pub(crate) struct Lifecycle {
    /// The node's description as clients see it.
    info: Arc<watch::Sender<NodeInfo>>,
    /// Cancelled to stop the node (the same token `Node::stop` cancels).
    stop: CancellationToken,
    runtime: Arc<SessionRuntime>,
    settings: Option<Weak<crate::settings::Settings>>,
}

impl Lifecycle {
    pub(crate) fn new(
        info: NodeInfo,
        stop: CancellationToken,
        runtime: Arc<SessionRuntime>,
    ) -> Self {
        Self {
            info: Arc::new(watch::channel(info).0),
            stop,
            runtime,
            settings: None,
        }
    }

    /// Live nodes attach their canonical mutation owner without a reference
    /// cycle. Standalone fact observers need no settings mutation authority.
    pub(crate) fn with_settings(mut self, settings: &Arc<crate::settings::Settings>) -> Self {
        self.settings = Some(Arc::downgrade(settings));
        self
    }

    fn seal_settings(&self) {
        if let Some(settings) = self.settings.as_ref().and_then(Weak::upgrade) {
            settings.seal();
        }
    }

    /// Cancelled when the node stops: for tasks of the service's own
    /// that should end with it (protocol 11's notices).
    pub(crate) fn stopping(&self) -> CancellationToken {
        self.stop.clone()
    }

    /// The node's current description.
    pub(crate) fn info(&self) -> NodeInfo {
        self.info.borrow().clone()
    }

    /// A receiver of the description, for the RPC server's `Welcome`.
    pub(crate) fn watch_info(&self) -> watch::Receiver<NodeInfo> {
        self.info.subscribe()
    }

    /// Whether new sessions are refused.
    pub(crate) fn is_draining(&self) -> bool {
        self.stop.is_cancelled() || self.info.borrow().state == NodeState::Draining
    }

    /// Carry out `node.stop`. Returns at once; the stop itself runs in the
    /// node's shutdown path.
    pub(crate) fn stop(&self, when: StopWhen) {
        self.seal_settings();
        match when {
            StopWhen::Now => {
                tracing::info!("stop requested");
                self.stop.cancel();
            }
            StopWhen::WhenIdle => {
                let started = self.info.send_if_modified(|info| {
                    let was = info.state;
                    info.state = NodeState::Draining;
                    was != NodeState::Draining
                });
                if started {
                    tracing::info!("draining: refusing new sessions, stopping once none is live");
                    drop(tokio::spawn(stop_when_idle(
                        Arc::clone(&self.runtime),
                        self.stop.clone(),
                    )));
                }
            }
        }
    }
}

/// Wait until no session is live, then stop the node. Ends early if the
/// node stops for another reason.
async fn stop_when_idle(runtime: Arc<SessionRuntime>, stop: CancellationToken) {
    loop {
        let live: Vec<SessionHandle> = runtime
            .list()
            .into_iter()
            .filter(|info| info.status.is_live())
            // A parked session holds no process: the node's restart
            // interrupts it as any live session, and nothing is lost by
            // not waiting for it (protocol 12).
            .filter(|info| info.parked.is_none())
            .filter_map(|info| runtime.get(info.id))
            .collect();
        if live.is_empty() {
            tracing::info!("drained: no live session left; stopping");
            stop.cancel();
            return;
        }
        for handle in live {
            tokio::select! {
                () = stop.cancelled() => return,
                _final = handle.ended() => {}
            }
        }
    }
}
