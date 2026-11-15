//! What sessions are created with, prepared once at node start: the
//! node's profiles, the model routes, the launch seam and the directories
//! sessions and probes use. Moved out of `node.rs` unchanged (Quality B),
//! which assembles the node from it.

use std::path::PathBuf;
use std::sync::Arc;

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

/// Configuration reads and downgrade backups stay off async runtime workers.
pub(super) async fn settings(
    config: &Config,
    runtime: &Arc<vessel_session::SessionRuntime>,
    tasks: &tokio_util::task::TaskTracker,
    stop: &tokio_util::sync::CancellationToken,
) -> Result<Arc<crate::settings::Settings>, crate::error::NodeError> {
    let data = config.data_dir.clone();
    let runtime = Arc::clone(runtime);
    let flag = vessel_proto::parking::ParkAfter::from_duration(config.park_after);
    let flag_given = config.park_after_given;
    let tasks = tasks.clone();
    let stop = stop.clone();
    tokio::task::spawn_blocking(move || {
        Arc::new(crate::settings::Settings::open(
            data, runtime, flag, flag_given, tasks, stop,
        ))
    })
    .await
    .map_err(crate::error::NodeError::Task)
}
