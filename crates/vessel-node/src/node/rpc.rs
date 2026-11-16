//! Construct the shared transport resources once, before any listener serves.

use std::sync::Arc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_rpc::{Server, ServerConfig};
use vessel_store::NodeToken;

use crate::{guard::Guarded, lifecycle::Lifecycle};

/// These are the original shared resources, not another connection registry.
pub(super) struct Rpc {
    pub(super) server: Arc<Server<Guarded>>,
    pub(super) token: Arc<NodeToken>,
    pub(super) closing: CancellationToken,
    pub(super) connections: TaskTracker,
}

impl Rpc {
    pub(super) fn new(
        service: Arc<Guarded>,
        lifecycle: &Lifecycle,
        config: &ServerConfig,
        token: NodeToken,
    ) -> Self {
        let closing = CancellationToken::new();
        let server = Arc::new(Server::new(
            service,
            lifecycle.watch_info(),
            config.clone(),
            closing.clone(),
        ));
        Self {
            server,
            token: Arc::new(token),
            closing,
            connections: TaskTracker::new(),
        }
    }
}
