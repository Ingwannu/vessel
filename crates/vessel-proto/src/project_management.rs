//! Project registration/presentation/settings, separate from the registry dependency.
//! No registration call creates a workspace, executes Git hooks or releases automation.

use crate::{ProtocolVersion, workspace::ProjectName};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Assigned at this project's wiring merge, independently of the registry dependency.
/// Keeping a separate package gate prevents older registry peers claiming newer project methods.
pub const PROJECT_MANAGEMENT: Option<ProtocolVersion> = None;

/// Register the exact selected canonical directory, including a nested Git subproject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RegisterProjectDirectory {
    /// Existing absolute directory on the selected node, not a client-local path.
    pub path: PathBuf,
    /// Optional explicit initial presentation; does not rename anything on disk.
    pub name: Option<ProjectName>,
}
