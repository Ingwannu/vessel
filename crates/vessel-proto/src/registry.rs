//! Independent project/workspace registry contracts (ADR 0082).
//! No IO or inferred cwd identity; calls remain unavailable until integration.

mod config;
mod project;
mod snapshot;
mod workspace;

pub use config::{
    ConfigDocument, ConfigRevision, ConfigText, ConfigTextError, ConfigValidity, MAX_CONFIG_BYTES,
    ReadProjectConfig, WriteProjectConfig,
};
pub use project::{
    CreateProjectDirectory, DirectoryName, DirectoryNameError, ProjectMetadata,
    ReadProjectMetadata, RenameProject,
};
pub use workspace::{
    ArchiveWorkspace, ArchiveWorkspaces, CreateWorkspace, GetWorkspace, ListWorkspaces,
    RegistryRevision, RenameWorkspace, RestoreWorkspace, WorkspaceBacking, WorkspaceDescriptor,
    WorkspaceIsolation, WorkspaceLifecycle, WorkspaceList, WorkspaceVisibility,
    WorkspaceVisibilityFilter,
};

pub use snapshot::{RegistrySnapshot, WorkspaceMembership, WorkspaceResource};

use crate::ProtocolVersion;

/// Assign only after the node lifecycle, durable receipts and all UI/CLI
/// consumers validate workspace ownership and partial cleanup outcomes.
/// None refuses registry calls before dispatch, including forwarded calls.
pub const PROJECT_WORKSPACE_REGISTRY: Option<ProtocolVersion> = None;
