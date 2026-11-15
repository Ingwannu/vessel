//! Locked startup writes, including the injection decision before node.id exists.

use super::{Prepared, SetupDirs, model_client};
use crate::attachments::staging;
use crate::error::NodeError;
use crate::fleet::FleetFiles;
use crate::models::Models;
use crate::profiles::NodeProfiles;
use crate::transfers;
use chrono::Utc;
use std::sync::Arc;
use vessel_harness::HarnessEnv;
use vessel_secure::NodeKeypair;
use vessel_store::{DataDir, NodeLock, NodeOwner};

/// The blocking part of start-up: directory, identity, token, key, lock.
pub(super) fn prepare(dir: &DataDir, env: HarnessEnv) -> Result<Prepared, NodeError> {
    dir.prepare().map_err(NodeError::DataDir)?;
    let owner = NodeOwner {
        pid: std::process::id(),
        executable: std::env::current_exe().unwrap_or_default(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        started_at: Utc::now(),
    };
    let lock = NodeLock::acquire(dir, &owner).map_err(NodeError::AlreadyRunning)?;
    dir.initialize_tool_settings().map_err(NodeError::DataDir)?;
    let id = dir.load_or_create_node_id().map_err(NodeError::DataDir)?;
    let token = dir.load_or_create_token().map_err(NodeError::DataDir)?;
    // Created on the first start of every node, new or existing (ADR 0009),
    // so its identity exists before any device pairs.
    let key = dir.load_or_create_node_key().map_err(NodeError::DataDir)?;
    let keypair = NodeKeypair::from_secret(*key.expose());
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
    Ok(Prepared {
        id,
        token,
        keypair: Arc::new(keypair),
        lock,
        profiles,
        models,
        dirs: SetupDirs {
            mcp: mcp_dir,
            probe: probe_dir,
            workspace: workspace_dir,
        },
    })
}
