//! Local socket/pipe layout is shared by node IPC and its desktop launcher.
use super::DataDir;
use std::path::PathBuf;
use vessel_proto::NodeId;

const SOCKET_FILE: &str = "vesseld.sock";

impl DataDir {
    /// The node's local socket on Unix (ADR 0007). Windows uses a named
    /// pipe instead, named after the node id by `vessel-node`.
    #[must_use]
    pub fn socket_path(&self) -> PathBuf {
        self.root.join(SOCKET_FILE)
    }

    /// The authenticated local endpoint for a known node identity. Keeping the
    /// Windows pipe spelling beside this directory layout prevents client and
    /// node launchers from independently inventing the endpoint.
    #[must_use]
    #[cfg(unix)]
    pub fn local_endpoint(&self, _node: NodeId) -> PathBuf {
        self.socket_path()
    }

    /// Windows uses one pipe spelling for the authenticated node identity.
    #[must_use]
    #[cfg(windows)]
    pub fn local_endpoint(&self, node: NodeId) -> PathBuf {
        PathBuf::from(format!(r"\\.\pipe\vessel-node-{node}"))
    }
}
