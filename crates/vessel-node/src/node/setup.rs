//! Bootstrap helpers prepared once at node start: the
//! node's profiles, the model routes, the launch seam and the directories
//! sessions and probes use. Moved out of `node.rs` unchanged (Quality B),
//! which assembles the node from it.

use crate::error::NodeError;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

use vessel_proto::NodeId;

use crate::config::Config;
use crate::fleet::FleetMcp;
use crate::ipc::Endpoint;
use crate::models::Models;
use crate::profiles::NodeProfiles;
use crate::profiles::launch::{AgentTools, Launcher, McpSource};

/// What sessions are created with: the node's profiles and the launch seam.
pub(super) struct SessionSetup {
    pub(super) profiles: Arc<NodeProfiles>,
    /// The model routes and the proxy's tokens (ADR 0015).
    pub(super) models: Arc<Models>,
    pub(super) launcher: Launcher,
    /// The fleet's MCP servers, which the fleet loop keeps current and the
    /// launcher asks for.
    pub(super) fleet_mcp: Arc<FleetMcp>,
    /// Where agents are probed (`DataDir::probe_dir`, protocol 9).
    pub(super) probe_dir: PathBuf,
    /// The owner-only MCP directory (`DataDir::prepare_mcp_dir`), which the
    /// session runtime also gives a direct driver's launches for their
    /// private files (`AgentSetup::private_dir`, ADR 0057).
    pub(super) private_dir: PathBuf,
    /// Worktrees and their setup logs (`DataDir::workspace_dir`,
    /// protocol 10).
    pub(super) workspace_dir: PathBuf,
}

/// The directories sessions and probes use, prepared under the lock.
pub(super) struct SetupDirs {
    /// Owner-only, emptied of a previous run's files (ADR 0012).
    pub(super) mcp: PathBuf,
    /// Where agents are probed (protocol 9).
    pub(super) probe: PathBuf,
    /// Worktrees, their setup logs and git's empty hooks folder
    /// (protocol 10).
    pub(super) workspace: PathBuf,
}

impl SessionSetup {
    pub(super) fn new(
        config: &Config,
        id: NodeId,
        profiles: NodeProfiles,
        models: Arc<Models>,
        SetupDirs {
            mcp,
            probe,
            workspace,
        }: SetupDirs,
    ) -> Self {
        let tools = config.vessel_mcp.clone().map(|mcp| AgentTools {
            program: mcp.program,
            args: mcp.args,
            socket: Endpoint::of(&config.data_dir, id).path().to_owned(),
        });
        let fleet_mcp = Arc::new(FleetMcp::default());
        let source: Arc<dyn McpSource> = Arc::<FleetMcp>::clone(&fleet_mcp);
        Self {
            profiles: Arc::new(profiles),
            models,
            private_dir: mcp.clone(),
            launcher: Launcher::new(mcp, source, tools),
            fleet_mcp,
            probe_dir: probe,
            workspace_dir: workspace,
        }
    }
}

/// The model proxy's listener, on a port of loopback's the system picks
/// (harnesses learn it from their environment). It is served from `core`'s
/// stop: model calls end with the node, a harness still calling gets its
/// connection closed, as its session is interrupted anyway.
pub(super) async fn model_proxy(models: Models) -> Result<(Arc<Models>, TcpListener), NodeError> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(NodeError::ModelProxy)?;
    models.listening(listener.local_addr().map_err(NodeError::ModelProxy)?);
    Ok((Arc::new(models), listener))
}

pub(super) fn model_client() -> Result<vessel_llm::client::Client, NodeError> {
    let limits = vessel_llm::client::Limits::default();
    vessel_llm::client::Client::new(limits)
        .or_else(|error| {
            // The target it had in node.rs before the move, so log filters
            // that select it keep working.
            tracing::warn!(target: "vessel_node::node", %error, "model calls to https providers will fail");
            vessel_llm::client::Client::with_roots(rustls::RootCertStore::empty(), limits)
        })
        .map_err(NodeError::ModelClient)
}

/// A failed registry never resets saved data or prevents unrelated node operations.
pub(super) async fn host_registry(
    config: &Config,
    stop: tokio_util::sync::CancellationToken,
    tasks: &tokio_util::task::TaskTracker,
) -> Option<crate::host_registry::HostRegistry> {
    #[cfg(test)]
    if let Some(registry) = &config.test_host_registry {
        return Some(registry.clone());
    }
    let _assigned_version = vessel_proto::host_registry::CLI_PARITY_PROTOCOL?;
    match crate::host_registry::HostRegistry::start(config.data_dir.clone(), stop, tasks).await {
        Ok(registry) => Some(registry),
        Err(_failure) => {
            tracing::warn!("host registry is unavailable; saved data is retained");
            None
        }
    }
}

pub(super) fn listening(id: NodeId, address: Option<SocketAddr>, local: &Endpoint) {
    tracing::info!(target: "vessel_node::node",
        node = %id,
        address = ?address,
        local = %local.path().display(),
        "node listening"
    );
}
