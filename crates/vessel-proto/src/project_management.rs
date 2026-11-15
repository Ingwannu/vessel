//! Project registration/presentation/settings, separate from the registry dependency.
//! No registration call creates a workspace, executes Git hooks or releases automation.

use crate::{ProtocolVersion, workspace::ProjectName};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use crate::registry::CreateProjectDirectory;

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

/// Filesystem creation succeeded; registration and its compensating cleanup are explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectDirectoryOutcome {
    /// One empty directory is now a registered project, without Git initialization.
    Created {
        /// The authoritative project identity and exact directory.
        project: crate::workspace::ProjectInfo,
    },
    /// Persistence or registration failed after this request made the directory.
    RegistrationFailed {
        /// The registration failure; clients branch on its typed code.
        error: crate::WireError,
        /// Whether the newly created directory could be removed without deleting contents.
        cleanup: ProjectDirectoryCleanup,
    },
}

/// Cleanup never deletes files or recurses into a directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectDirectoryCleanup {
    /// The single empty directory was removed.
    Removed,
    /// A path remains, for example because another process populated it.
    Retained {
        /// Exact node-local path to show for manual recovery.
        path: PathBuf,
        /// Why nonrecursive cleanup was refused.
        error: crate::WireError,
    },
}
