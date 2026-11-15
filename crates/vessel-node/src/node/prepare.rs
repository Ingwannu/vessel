//! Blocking node preparation under the data-directory lock. Extracted before
//! provider initialization so startup assembly keeps its existing size budget.

use super::setup::{SetupDirs, model_client};
use crate::{
    attachments::staging, config::Config, error::NodeError, fleet::FleetFiles, models::Models,
    profiles::NodeProfiles, transfers,
};
use chrono::Utc;
use std::sync::Arc;
use vessel_harness::HarnessEnv;
use vessel_proto::NodeId;
use vessel_secure::NodeKeypair;
use vessel_store::{DataDir, NodeLock, NodeOwner, NodeToken};

/// What the blocking part of start-up produces.
pub(super) struct Prepared {
    pub(super) id: NodeId,
    pub(super) token: NodeToken,
    /// From `node.key` (ADR 0009), shared by the gateway and the relay
    /// links.
    pub(super) keypair: Arc<NodeKeypair>,
    pub(super) lock: NodeLock,
    /// From `profiles.toml` (ADR 0011).
    pub(super) profiles: NodeProfiles,
    /// From `models.toml` (ADR 0015).
    pub(super) models: Models,
    /// The directories sessions, probes and worktrees use.
    pub(super) dirs: SetupDirs,
    pub(super) providers: crate::providers::PreparedProviders,
}

/// The blocking part of start-up: directory, identity, token, key, lock.
fn prepare(dir: &DataDir, env: HarnessEnv) -> Result<Prepared, NodeError> {
    dir.prepare().map_err(NodeError::DataDir)?;
    let id = dir.load_or_create_node_id().map_err(NodeError::DataDir)?;
    let token = dir.load_or_create_token().map_err(NodeError::DataDir)?;
    // Created on the first start of every node, new or existing (ADR 0009),
    // so its identity exists before any device pairs.
    let key = dir.load_or_create_node_key().map_err(NodeError::DataDir)?;
    let keypair = NodeKeypair::from_secret(*key.expose());
    let owner = NodeOwner {
        pid: std::process::id(),
        executable: std::env::current_exe().unwrap_or_default(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        started_at: Utc::now(),
    };
    let lock = NodeLock::acquire(dir, &owner).map_err(NodeError::AlreadyRunning)?;
    // After the lock: the MCP directory is emptied, which only the node
    // owning the data directory may do.
    let mcp_dir = dir.prepare_mcp_dir().map_err(NodeError::McpDir)?;
    let probe_dir = dir.prepare_probe_dir().map_err(NodeError::ProbeDir)?;
    let workspace_dir = dir
        .prepare_workspace_dir()
        .map_err(NodeError::WorkspaceDir)?;
    // Transfers expire at restart (protocol 6), and so do uploads (protocol
    // 10); same reason to wait for the lock.
    transfers::clear(dir);
    staging::clear(dir);
    let profiles = NodeProfiles::load(dir.clone(), env).map_err(NodeError::Profiles)?;
    let models = Models::load(dir.clone(), FleetFiles::new(dir), model_client()?)
        .map_err(NodeError::Models)?;
    // A profile naming a route that is not there is not fatal: its
    // sessions are refused, with the route named, until the file has it.
    models.check_profiles(&profiles.list(None));
    let providers =
        crate::providers::PreparedProviders::read(dir.clone()).map_err(NodeError::Providers)?;
    Ok(Prepared {
        id,
        token,
        keypair: Arc::new(keypair),
        lock,
        profiles,
        models,
        providers,
        dirs: SetupDirs {
            mcp: mcp_dir,
            probe: probe_dir,
            workspace: workspace_dir,
        },
    })
}

/// The client model calls go out with: the operating system's roots, or
/// none (with a warning) on a machine that has none, where only `http`
/// providers (a local server) can then be reached.
/// [`prepare`], on a thread that may block.
pub(super) async fn prepare_off_thread(config: &Config) -> Result<Prepared, NodeError> {
    let dir = config.data_dir.clone();
    let env = config.harness_env.clone();
    tokio::task::spawn_blocking(move || prepare(&dir, env))
        .await
        .map_err(NodeError::Task)?
}
