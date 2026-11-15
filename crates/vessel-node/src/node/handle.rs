//! A running node's public observations and shutdown, separate from assembly.

use std::net::SocketAddr;
use std::path::Path;
use tokio::sync::watch;
use vessel_proto::{NodeInfo, RouteUrl};
use vessel_relay::LinkState;

use super::Node;
use crate::NodeError;

impl Node {
    /// The TCP address the node listens on (with the real port when the
    /// config asked for port 0); `None` when it listens on its local socket
    /// only.
    #[must_use]
    pub const fn address(&self) -> Option<SocketAddr> {
        self.address
    }

    /// The node's local socket (Unix) or named pipe (Windows), which
    /// `vesseld attach` connects to.
    #[must_use]
    pub fn local_endpoint(&self) -> &Path {
        self.local.path()
    }

    /// Each configured relay and where the node's link to it stands
    /// (connecting, registered, refused), to read or wait on.
    #[must_use]
    pub fn relay_links(&self) -> Vec<(RouteUrl, watch::Receiver<LinkState>)> {
        self.relays.states()
    }

    /// The node's current description.
    #[must_use]
    pub fn info(&self) -> NodeInfo {
        self.lifecycle.info()
    }

    /// Resolves once the node was asked to stop, by [`Node::stop`] or by a
    /// client (`node.stop`, protocol 3). The caller then calls
    /// [`Node::stop`] to wait for the shutdown to finish.
    pub fn stop_requested(&self) -> impl Future<Output = ()> + Send + 'static {
        self.stop.clone().cancelled_owned()
    }

    /// Stop accepting, end every session as `Interrupted`, commit their
    /// logs, close connections and the store, and release the data
    /// directory; returns when all of that is done.
    ///
    /// # Errors
    /// The HTTP server had failed.
    pub async fn stop(self) -> Result<(), NodeError> {
        self.stop.cancel();
        self.task.await.map_err(NodeError::Task)?
    }
}
